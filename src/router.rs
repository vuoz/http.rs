use crate::{response::IntoResp, Request};
use async_std::sync::Arc;
use std::cell::{Cell, RefCell};
use std::io::Result;
use std::pin::Pin;
use std::rc::Rc;
use std::{collections::HashMap, future::Future};
use tokio::net::TcpListener;

pub type HandlerResponse<'a> = Pin<Box<dyn Future<Output = Box<dyn IntoResp + Send>> + Send + 'a>>;

pub type HandlerType = fn(Request) -> HandlerResponse<'static>;
//Still need to implement the extractor for the state
//pub type HandlerTypeExp<T> = fn(RequestWithState<T>) -> HandlerResponse<'static>;

#[derive(Debug)]
pub struct Node {
    pub subpath: String,
    pub children: Option<Box<Vec<Box<Node>>>>,
    pub handler: Option<HandlerType>,
}
impl Node {
    pub fn new() -> Self {
        Node {
            subpath: "/".to_string(),
            children: None,
            handler: None,
        }
    }
    pub fn get_handler(self, path: String) -> Option<HandlerType> {
        if path == "/" {
            return self.handler;
        }
        let children = self.children;
        match pub_walk(children, path) {
            Some(handler) => return Some(handler),
            None => return None,
        }
    }
    pub fn add_handler(&mut self, path: String, handler: HandlerType) -> Result<&mut Self> {
        if path == "/" {
            self.handler = Some(handler);
            return Ok(self);
        }
        let res = pub_walk_return_node(self, path, handler, 0);
        if let Some(()) = res {
            println!("returned some");
            return Ok(self);
        } else {
            println!("returned none");
            return Ok(self);
        }
    }
}
fn pub_walk_return_node(node: &mut Node, path: String, func: HandlerType, i_: u32) -> Option<()> {
    match node.children.as_mut() {
        Some(children) => {
            for i in 0..children.len() {
                let child = children.get_mut(i)?;
                if child.subpath == path {
                    return None;
                }
                if path.contains(child.subpath.as_str()) {
                    match pub_walk_return_node(child, path.clone(), func, i_) {
                        Some(_) => (),
                        None => (),
                    };
                }
            }
        }
        None => {
            let mut path_rn = node.subpath.clone();
            let mut currnode = node;
            let path_to_add: Vec<String> = path
                .split(path_rn.as_str())
                .take_while(|split| split.to_string() != "")
                .map(|split| split.to_string())
                .collect();
            for i in path_to_add.into_iter() {
                let new_node = Node {
                    subpath: path_rn.clone() + "/" + i.as_str(),
                    handler: None,
                    children: None,
                };
                let new_sub_path = format!("{}{}", "/", i);
                path_rn += new_sub_path.as_str();

                let mut new_vec = Vec::new();
                let box_node = Box::new(new_node);
                new_vec.push(box_node);
                let boxed_vec = Box::new(new_vec);
                currnode.children = Some(boxed_vec);
                //currnode = &mut new_node;
            }
        }
    };

    return None;
}
fn pub_walk(children: Option<Box<Vec<Box<Node>>>>, path: String) -> Option<HandlerType> {
    if let Some(children) = children {
        for child in children.into_iter() {
            if child.subpath == path {
                return child.handler;
            }
            if path.contains(child.subpath.as_str()) {
                let new_children = child.children;
                let handler = match pub_walk(new_children, path.clone()) {
                    Some(handler) => handler,
                    None => return None,
                };
                return Some(handler);
            }
            return None;
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
