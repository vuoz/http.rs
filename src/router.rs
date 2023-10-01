use crate::request::RouteExtract;

use crate::{response::IntoResp, Request};
use async_std::sync::Arc;
use http::StatusCode;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::io::Result;
use std::mem::replace;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::rc::Rc;
use std::{collections::HashMap, future::Future};
use tokio::net::TcpListener;

pub type HandlerResponse<'a> = Pin<Box<dyn Future<Output = Box<dyn IntoResp + Send>> + Send + 'a>>;

pub type HandlerType = fn(Request) -> HandlerResponse<'static>;
pub type HandlerTypeState<T> = fn(Request, T) -> HandlerResponse<'static>;
#[derive(Debug, Default, Clone, Copy)]
pub enum Handler<T: std::clone::Clone + std::marker::Copy> {
    #[default]
    None,
    Without(HandlerType),
    WithState(HandlerTypeState<T>),
}
impl<T: std::clone::Clone + std::marker::Copy> Handler<T>
where
    T: Clone,
{
    pub async fn handle(self, req: Request, state: Option<T>) -> Option<Box<dyn IntoResp + Send>> {
        match self {
            Handler::Without(func) => Some(func(req).await),
            Handler::WithState(func) => match state {
                Some(state) => Some(func(req, state).await),
                None => Some(Box::new((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Missing state".to_string(),
                )) as Box<dyn IntoResp + Send>),
            },
            Self::None => None,
        }
    }
}

pub struct RoutingResult<T: std::clone::Clone + std::marker::Copy> {
    pub handler: Handler<T>,
    pub extract: Option<RouteExtract>,
}
#[derive(Debug, Default)]
pub struct Node<T: Clone + Default + Send + std::marker::Copy + std::marker::Sync> {
    pub subpath: String,
    pub children: Option<Box<Vec<Box<Node<T>>>>>,
    pub handler: Option<Handler<T>>,
    pub state: Option<T>,
}
impl<T> Node<T>
where
    T: Sync,
    T: Clone,
    T: Default,
    T: Send,
    T: Copy,
{
    pub fn new(path: String) -> Self {
        Node {
            subpath: path,
            children: None,
            handler: None,
            state: None,
        }
    }
    pub fn add_state(&mut self, state: &mut T) -> Self {
        self.state = Some(*state);
        return std::mem::take(self);
    }
    pub async fn serve(&'static self, addr: String) -> ! {
        let listener = match TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(e) => panic!("Cannot create listener Error: {e} "),
        };
        loop {
            let (socket, _) = match listener.accept().await {
                Ok((socket, other_thing)) => (socket, other_thing),
                Err(e) => panic!("Canot accept connection Error: {e}"),
            };
            tokio::spawn(async move {
                match crate::handle_conn_node_based(socket, &self, None, self.state.clone()).await {
                    Ok(_) => (),
                    Err(e) => {
                        panic!("Cannot handle incomming connection: {e}")
                    }
                };
            });
        }
    }
    pub fn get_handler(&self, path: String) -> Option<RoutingResult<T>> {
        if path == "/" {
            match &self.handler {
                Some(handler) => {
                    return Some(RoutingResult {
                        handler: handler.clone(),
                        extract: None,
                    })
                }
                None => return None,
            }
        }
        match pub_walk(&self.children, path) {
            Some(handler) => return Some(handler),
            None => return None,
        }
    }
    pub fn add_handler(
        &mut self,
        path: String,
        handler: Handler<T>,
    ) -> std::result::Result<Box<Self>, ()> {
        if path == "/" {
            self.handler = Some(handler);
            return Ok(Box::new(std::mem::take(self)));
        }
        let res = pub_walk_add_node(self, path, handler);
        if let Some(node) = res {
            return Ok(node);
        } else {
            return Ok(Box::new(std::mem::take(self)));
        }
    }
    pub fn insert(&mut self, path: String, pathRn: String, func: Handler<T>) -> Box<Node<T>> {
        //This is the base case when the path is reached the node is returned
        if path == pathRn {
            self.handler = Some(func);
            return Box::new(std::mem::take(self));
        }

        let path_for_new_node = match pathRn.as_str() {
            "/" => {
                let splits: Vec<String> = path.split("/").map(|split| split.to_string()).collect();
                let mut to_add = String::new();
                for i in splits.into_iter() {
                    if i == "" {
                        continue;
                    }
                    to_add = i.clone();
                    break;
                }
                let final_str = "/".to_string() + to_add.as_str();
                final_str
            }
            _ => {
                let missing_part_of_path = path.replace(pathRn.clone().as_str(), "").to_string();

                let splits: Vec<String> = missing_part_of_path
                    .split("/")
                    .map(|split| split.to_string())
                    .collect();
                let mut to_add_to_curr = String::new();
                for i in splits.into_iter() {
                    if i == "" {
                        continue;
                    }
                    to_add_to_curr = i.clone();
                    break;
                }
                let mut final_path = match pathRn.ends_with("/") {
                    false => pathRn.clone() + "/" + to_add_to_curr.as_str(),
                    true => pathRn.clone() + to_add_to_curr.as_str(),
                };
                if to_add_to_curr == "" {
                    final_path = missing_part_of_path;
                }
                final_path
            }
        };
        let mut new_node = Node::new(path_for_new_node.clone());
        match self.children.as_mut() {
            Some(children) => {
                let node = new_node.insert(path.clone(), path_for_new_node.clone(), func);
                children.push(node);
                return Box::new(std::mem::take(self));
            }
            None => {
                let node = new_node.insert(path.clone(), path_for_new_node.clone(), func);
                let mut new_vec = Vec::new();
                new_vec.push(node);
                let boxed_vec = Box::new(new_vec);
                self.children = Some(boxed_vec);
                return Box::new(std::mem::take(self));
            }
        };
    }
}
fn pub_walk_add_node<
    T: std::default::Default
        + std::clone::Clone
        + std::marker::Send
        + std::marker::Copy
        + std::marker::Sync,
>(
    node: &mut Node<T>,
    path: String,
    func: Handler<T>,
) -> Option<(Box<Node<T>>)> {
    match node.children.as_mut() {
        Some(children) => {
            let mut matches = 0;
            for i in 0..children.len() {
                let child = children.get_mut(i)?;
                if child.subpath == path {
                    child.handler = Some(func);
                    return Some(Box::new(std::mem::take(node)));
                }
                let test_str = child.subpath.clone() + "/";
                if path.contains(test_str.as_str()) {
                    // This causes a bug since /wow also matches on /wowo
                    // this is not wanted since you obv should not append to /wowo
                    matches = matches + 1;

                    match pub_walk_add_node(child, path.clone(), func) {
                        Some(node) => return Some(node),
                        None => return None,
                    };
                }
            }
            if matches == 0 {
                let node_path_curr = node.subpath.clone();
                let node = node.insert(path, node_path_curr, func);
                return Some(node);
            }
            return None;
        }
        None => {
            let node_path_curr = node.subpath.clone();
            let node = node.insert(path, node_path_curr, func);
            return Some(node);
        }
    }
}

fn pub_walk<
    T: std::default::Default
        + std::clone::Clone
        + std::marker::Send
        + std::marker::Copy
        + std::marker::Sync,
>(
    children: &Option<Box<Vec<Box<Node<T>>>>>,
    path: String,
) -> Option<RoutingResult<T>> {
    if let Some(children) = children {
        for child in children.as_ref().into_iter() {
            if child.subpath.contains(":") {
                let splits: Vec<String> = child
                    .subpath
                    .split(":")
                    .map(|split| split.to_string())
                    .collect();
                if splits.len() != 2 {
                    continue;
                }
                // This is the identifier to the extract. For instance if we registered the route
                // /user/:id then split by ":"  at index 0 we get the path before the ":" and at index 1 the identifier
                // or how the user should be abled to extract it in his handler
                let identifier = splits.get(1)?;
                let path_before = splits.get(0)?;
                if path.contains(path_before) {
                    let values_split: Vec<String> = path
                        .split(path_before)
                        .map(|split| split.to_string())
                        .collect();
                    if values_split.len() != 2 {
                        continue;
                    }
                    let value = values_split.get(1)?;
                    // This will likely become a HashMap since we
                    // want to have to ability to handle multiple extractors
                    let new_extract = RouteExtract {
                        value: value.clone(),
                        identifier: identifier.clone(),
                    };
                    match child.handler {
                        Some(handler) => {
                            return Some(RoutingResult {
                                handler,
                                extract: Some(new_extract),
                            })
                        }
                        None => (),
                    }
                    println!("In here found the path  ident: {}", identifier);
                }
            }
            if child.subpath == path {
                match child.handler {
                    Some(handler) => {
                        return Some(RoutingResult {
                            handler,
                            extract: None,
                        })
                    }
                    None => (),
                }
            }
            if path.contains(child.subpath.as_str()) {
                let new_children = &child.children;
                match pub_walk(new_children, path.clone()) {
                    Some(handler) => return Some(handler),
                    None => (),
                };
            }
        }
    }
    None
}

#[derive(Clone, Debug)]
pub struct Router<T: Clone> {
    routes: HashMap<String, HandlerType>,
    fallback: Option<HandlerType>,
    state: Option<T>,
}

impl<T: Clone> Router<T> {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            fallback: None,
            state: None,
        }
    }
    pub async fn handle(&mut self, path: &str, func: HandlerType) -> std::io::Result<Self> {
        self.routes.insert(path.to_string(), func);
        return Ok(self.clone());
    }
    pub fn with_fallback(&mut self, func: HandlerType) -> std::io::Result<Self> {
        self.fallback = Some(func);
        return Ok(self.clone());
    }
    pub fn add_state(&mut self, state: T) -> Self {
        self.state = Some(state);
        self.clone()
    }

    pub async fn serve(self, addr: String) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr).await?;

        loop {
            let (socket, _) = listener.accept().await?;
            let routes_clone = self.routes.clone();
            let fallback_clone = self.fallback.clone();
            tokio::spawn(async move {
                match crate::handle_conn(socket, Arc::new(routes_clone), fallback_clone).await {
                    Ok(_) => (),
                    Err(e) => {
                        panic!("Cannot handle incomming connection: {e}")
                    }
                };
            });
        }
    }
}
