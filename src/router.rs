use crate::{response::IntoResp, Request};
use async_std::sync::Arc;
use std::pin::Pin;
use std::{collections::HashMap, future::Future};
use tokio::net::TcpListener;

pub type HandlerResponse<'a> = Pin<Box<dyn Future<Output = Box<dyn IntoResp + Send>> + Send + 'a>>;

pub type HandlerType = fn(Request) -> HandlerResponse<'static>;
//Still need to implement the extractor for the state
//pub type HandlerTypeExp<T> = fn(RequestWithState<T>) -> HandlerResponse<'static>;

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
