use crate::{response::IntoResp, Request};
use async_std::sync::Arc;
use std::future::{ready, Ready};
use std::pin::Pin;
use std::{
    collections::HashMap,
    future::{Future, IntoFuture},
};
use tokio::net::TcpListener;

pub type HandlerResponse<'a> = Pin<Box<dyn Future<Output = Box<dyn IntoResp + Send>> + Send + 'a>>;

pub type HandlerType = fn(Request) -> HandlerResponse<'static>;

#[derive(Clone)]
pub struct Router {
    pub routes: HashMap<String, HandlerType>,
    pub fallback: Option<HandlerType>,
}

impl Router {
    pub fn new() -> Router {
        Router {
            routes: HashMap::new(),
            fallback: None,
        }
    }
    pub async fn handle(&mut self, path: &str, func: HandlerType) -> std::io::Result<Router> {
        self.routes.insert(path.to_string(), func);
        return Ok(self.clone());
    }
    pub fn with_fallback(&mut self, func: HandlerType) -> std::io::Result<Router> {
        self.fallback = Some(func);
        return Ok(self.clone());
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
