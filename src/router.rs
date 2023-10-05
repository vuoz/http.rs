use crate::request::RouteExtract;

use crate::{response::IntoResp, Request};
use async_std::sync::Arc;
use http::StatusCode;
use std::hash::Hasher;
use std::pin::Pin;
use std::{collections::HashMap, future::Future};
use tokio::net::TcpListener;

pub type HandlerResponse<'a> = Pin<Box<dyn Future<Output = Box<dyn IntoResp + Send>> + Send + 'a>>;

pub type HandlerType = fn(Request) -> HandlerResponse<'static>;
pub type HandlerTypeState<T> = fn(Request, T) -> HandlerResponse<'static>;
pub type HandlerTypeStateAndExtract<T> = fn(Request, T) -> HandlerResponse<'static>;
pub type HandlerTypeWithStateAndMiddlewareExtract<T, S> =
    fn(Request, T, S) -> HandlerResponse<'static>;
#[derive(Debug, Default, Clone, Copy)]
pub enum Handler<T: std::clone::Clone> {
    #[default]
    None,
    Without(HandlerType),
    WithState(HandlerTypeState<T>),
    WithStateAndBodyExtract(HandlerTypeStateAndExtract<T>),
    WithMiddleware(HandlerTypeWithStateAndMiddlewareExtract<T>),
}
impl<T: std::clone::Clone> Handler<T>
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
            // Still need to implement this.
            Handler::WithStateAndBodyExtract(func) => return None,
            Self::None => None,
        }
    }
}

pub struct RoutingResult<T: std::clone::Clone> {
    pub handler: Handler<T>,
    pub extract: Option<HashMap<String, String>>,
}
#[derive(Debug, Default)]
pub struct Node<T: Clone + Default + Send + std::marker::Sync> {
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
{
    pub fn new(path: String) -> Self {
        Node {
            subpath: path,
            children: None,
            handler: None,
            state: None,
        }
    }
    pub fn add_state(&mut self, state: T) -> Self {
        self.state = Some(state);
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
    pub fn insert(&mut self, path: String, path_rn: String, func: Handler<T>) -> Box<Node<T>> {
        //This is the base case when the path is reached the node is returned
        if path == path_rn {
            self.handler = Some(func);
            return Box::new(std::mem::take(self));
        }

        let path_for_new_node = match path_rn.as_str() {
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
                let missing_part_of_path = path.replace(path_rn.clone().as_str(), "").to_string();

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
                let mut final_path = match path_rn.ends_with("/") {
                    false => path_rn.clone() + "/" + to_add_to_curr.as_str(),
                    true => path_rn.clone() + to_add_to_curr.as_str(),
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
    T: std::default::Default + std::clone::Clone + std::marker::Send + std::marker::Sync,
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
    T: std::default::Default + std::clone::Clone + std::marker::Send + std::marker::Sync,
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
                    //This path has more than one generic extract
                    // for example /user/:id/time/:ts

                    let child_path_splits: Vec<String> = child
                        .subpath
                        .split("/")
                        .map(|split| split.to_string())
                        .collect();
                    let curr_path_split: Vec<String> =
                        path.split("/").map(|split| split.to_string()).collect();
                    if curr_path_split.len() != child_path_splits.len() {
                        // if the path is longer than the one rn we continue the search
                        match pub_walk(&child.children, path.clone()) {
                            Some(handler) => return Some(handler),
                            None => (),
                        }
                        continue;
                    }
                    // switched the extracts to be an HashMap since it is now more than one
                    // extract
                    let mut extracts: HashMap<String, String> = HashMap::new();
                    'inner: for (i, val) in child_path_splits.into_iter().enumerate() {
                        if val.starts_with(":") {
                            let extract = curr_path_split.get(i)?;
                            extracts.insert(val.replace(":", "").to_string(), extract.to_string());
                            continue 'inner;
                        }
                    }
                    let handler = match &child.handler {
                        Some(handler) => handler,
                        None => {
                            continue;
                        }
                    };
                    return Some(RoutingResult {
                        handler: handler.clone(),
                        extract: Some(extracts),
                    });
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

                    let mut extracts = HashMap::new();
                    extracts.insert(identifier.clone(), value.clone());
                    match &child.handler {
                        Some(handler) => {
                            return Some(RoutingResult {
                                handler: handler.clone(),
                                extract: Some(extracts),
                            });
                        }
                        None => {
                            match pub_walk(&child.children, path.clone()) {
                                Some(handler) => return Some(handler),
                                None => (),
                            };
                            ()
                        }
                    }
                }
            }
            if child.subpath == path {
                match &child.handler {
                    Some(handler) => {
                        return Some(RoutingResult {
                            handler: handler.clone(),
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
