use crate::{response::IntoResp, Request};
use async_std::sync::Arc;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
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
    pub fn add_handler(
        &mut self,
        path: String,
        handler: HandlerType,
    ) -> std::result::Result<&mut Self, ()> {
        if path == "/" {
            self.handler = Some(handler);
            return Ok(self);
        }
        let res = pub_walk_add_node(self, path, handler);
        if let Some(()) = res {
            return Ok(self);
        } else {
            return Ok(self);
        }
    }
}
fn pub_walk_add_node(node: &mut Node, path: String, func: HandlerType) -> Option<()> {
    match node.children.as_mut() {
        Some(children) => {
            let mut matches = 0;
            for i in 0..children.len() {
                let child = children.get_mut(i)?;
                if child.subpath == path {
                    child.handler = Some(func);
                    return Some(());
                }
                let test_str = child.subpath.clone() + "/";
                if path.contains(test_str.as_str()) {
                    // This causes a bug since /wow also matches on /wowo
                    // this is not wanted since you obv should not append to /wowo
                    matches = matches + 1;

                    match pub_walk_add_node(child, path.clone(), func) {
                        Some(_) => return Some(()),
                        None => return None,
                    };
                }
            }
            if matches == 0 {
                match insert_node(node, path.clone(), func) {
                    Some(_) => return Some(()),
                    None => return None,
                }
            }
            return None;
        }
        None => {
            match insert_node(node, path, func) {
                Some(_) => return Some(()),
                None => return None,
            };
        }
    }
}
fn insert_node(node: &mut Node, path: String, func: HandlerType) -> Option<()> {
    let mut path_current = node.subpath.clone();
    let path_test: Vec<String> = path
        .split(path_current.as_str())
        .map(|split| split.to_string())
        .collect();

    let mut new_nodes_vec: VecDeque<Node> = VecDeque::new();
    for path in path_test.into_iter() {
        if path == "" {
            continue;
        }
        let new_path = match path_current.ends_with("/") {
            true => path_current + path.as_str(),
            false => path_current + "/" + path.as_str(),
        };
        path_current = new_path.clone();
        let new_node = Node {
            subpath: new_path,
            handler: None,
            children: None,
        };

        new_nodes_vec.push_back(new_node)
    }

    //there are some issues adding more than a double nested routes will fix in the
    //comming commits
    let mut finished_node = Vec::new();
    for i in 0..new_nodes_vec.len() {
        if i + 1 >= new_nodes_vec.len() {
            break;
        }
        let mut node_1 = new_nodes_vec.pop_front()?;
        let mut node_2 = new_nodes_vec.pop_front()?;
        if i == new_nodes_vec.len() {
            node_2.handler = Some(func);
        }
        let mut new_vec = Vec::new();
        new_vec.push(Box::new(node_2));
        let boxed_vec = Box::new(new_vec);
        node_1.children = Some(boxed_vec);
        if i == 0 {
            let boxed_node = Box::new(node_1);
            finished_node.push(boxed_node);
        }
    }
    let last_node = finished_node.pop()?;
    if let Some(children) = node.children.as_mut() {
        children.push(last_node);
        return Some(());
    }
    let mut new_vec = Vec::new();
    new_vec.push(last_node);
    let boxed_vec = Box::new(new_vec);
    node.children = Some(boxed_vec);

    Some(())
}
fn pub_walk(children: Option<Box<Vec<Box<Node>>>>, path: String) -> Option<HandlerType> {
    print!("In Search");
    if let Some(children) = children {
        for child in children.into_iter() {
            if child.subpath == path {
                return child.handler;
            }
            if path.contains(child.subpath.as_str()) {
                let new_children = child.children;
                match pub_walk(new_children, path.clone()) {
                    Some(handler) => return Some(handler),
                    None => return None,
                };
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
