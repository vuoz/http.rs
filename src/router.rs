use crate::{response::IntoResp, Request};
use async_std::sync::Arc;
use std::future::{ready, Ready};
use std::pin::Pin;
use std::{
    collections::HashMap,
    future::{Future, IntoFuture},
};
use tokio::net::TcpListener;
pub type FnResponse = dyn Future<Output = Box<dyn IntoResp>>;
pub type AltHandlerFunc = fn(Request) -> Pin<Box<FnResponse>>;
pub struct HandlerFuncReal(AltHandlerFunc);
impl std::marker::Send for FnResponse {}
impl IntoFuture for HandlerFuncReal {
    type Output = AltHandlerFunc;
    type IntoFuture = Ready<Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        ready(self.0)
    }
}
pub struct Router<T: Copy> {
    pub routes: HashMap<String, Arc<AltHandlerFunc>>,
    pub state: Option<T>,
}

impl<T: Copy + std::marker::Send + std::marker::Sync> Router<T> {
    pub fn new() -> Router<T> {
        Router {
            routes: HashMap::new(),
            state: None,
        }
    }
    pub async fn handle(
        &mut self,
        path: &str,
        func: HandlerFuncReal,
    ) -> std::io::Result<&mut Router<T>> {
        self.routes.insert(path.to_string(), Arc::new(func.0));
        return Ok(self);
    }
    pub fn add_state(&mut self, state: T) {
        self.state = Some(state);
    }
    pub async fn serve(&self, addr: String) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr).await?;

        loop {
            let (socket, _) = listener.accept().await?;
            tokio::spawn(async move {
                match crate::handle_conn(socket, self.routes).await {
                    Ok(_) => (),
                    Err(e) => {
                        panic!("Cannot handle incomming connection: {e}")
                    }
                };
            });
        }
    }
}
