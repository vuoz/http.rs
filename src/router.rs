use std::collections::HashMap;
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
pub struct Router<T> {
    pub routes: HashMap<String, String>,
    pub state: Option<T>,
}

impl<T> Router<T> {
    pub fn new() -> Router<T> {
        Router {
            routes: HashMap::new(),
            state: None,
        }
    }
    pub fn handle(&mut self, path: String, func: String) -> std::io::Result<()> {
        self.routes.insert(path, func);
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
