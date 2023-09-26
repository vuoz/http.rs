use crate::{response::IntoResp, Request};
use async_std::sync::Arc;
use std::io::Result;
use std::pin::Pin;
use std::{collections::HashMap, future::Future};
use tokio::net::TcpListener;

pub type HandlerResponse<'a> = Pin<Box<dyn Future<Output = Box<dyn IntoResp + Send>> + Send + 'a>>;

pub type HandlerType = fn(Request) -> HandlerResponse<'static>;
//Still need to implement the extractor for the state
//pub type HandlerTypeExp<T> = fn(RequestWithState<T>) -> HandlerResponse<'static>;

//This is an idea of a node base router and its singature. Will implement this over the next
//commits
pub struct RouterRoot {
    pub root: String,
    pub children: Option<Vec<Arc<Node>>>,
    pub handler: HandlerType,
}
pub struct Node {
    pub val: String,
    pub next: Option<Arc<Node>>,
    pub handler: HandlerType,
}
impl RouterRoot {
    pub fn get_handler(self, path: String) -> Option<HandlerType> {
        match self.walk(path) {
            Some(handler) => return Some(handler),
            None => return None,
        }
    }
    pub fn add_handler(&mut self, path: String) -> Result<()> {
        return Ok(());
    }
    fn walk(self, inpt: String) -> Option<HandlerType> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct Router<T: Clone> {
    // This router isn't really capable, since it does not support any
    // regex based routing and is not abled to nest routes.
    // In the Future a implementation that is more like a tree structure could help with that,
    // but that requires extractors to work since there wouldn't be any need for a such a router
    // if you can not extract the path
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
