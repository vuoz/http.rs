use async_std::sync::Arc;
use std::collections::HashMap;
use tokio::net::TcpListener;

use crate::response::IntoResp;

pub struct Router<T: Copy> {
    pub routes: HashMap<String, Arc<dyn IntoResp + Send + Sync>>,
    pub state: Option<T>,
}

impl<T: Copy> Router<T> {
    pub fn new() -> Router<T> {
        Router {
            routes: HashMap::new(),
            state: None,
        }
    }
    pub fn handle(
        &mut self,
        path: String,
        func: impl IntoResp + std::marker::Sync + std::marker::Send + 'static,
    ) -> std::io::Result<()> {
        self.routes.insert(path, Arc::new(func));
        return Ok(());
    }
    pub fn add_state(&mut self, state: T) {
        self.state = Some(state);
    }
    pub async fn serve(&self, addr: String) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr).await?;

        loop {
            let (socket, _) = listener.accept().await?;
            tokio::spawn(async move {
                match crate::handle_conn(socket).await {
                    Ok(_) => (),
                    Err(e) => {
                        println!("{e}");
                        return;
                    }
                };
            });
        }
    }
}
