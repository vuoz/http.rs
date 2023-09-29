pub mod parse;
pub mod request;
pub mod response;
pub mod router;
use crate::router::Node;
use http::StatusCode;
use request::parse_request;
use response::IntoResp;
use router::HandlerResponse;
use router::HandlerType;
use router::Router;
use std::collections::HashMap;
use std::io;

use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
#[derive(Debug)]
pub enum HandleConnError {
    Error(std::io::Error),
}
impl From<std::io::Error> for HandleConnError {
    fn from(value: std::io::Error) -> Self {
        HandleConnError::Error(value)
    }
}
#[derive(Debug)]
pub struct Header {
    pub key: String,
    pub val: String,
}
#[derive(Debug)]
pub enum Body {
    Binary(Vec<u8>),
    Text(String),
    None,
}
#[derive(Debug)]
pub enum ContentType {
    Json(String),
    UrlEncoded(HashMap<String, String>),
    PlainText(String),
    Binary(Vec<u8>),
    None,
}
#[derive(Debug)]
pub struct QueryParam {
    pub key: String,
    pub val: String,
}

pub enum TypeOfData {
    Header(Header),
    Body(Body),
}

#[derive(Debug, Clone)]
pub struct MetaData {
    pub method: String,
    pub path: String,
    pub version: String,
}
#[derive(Debug)]
pub struct Request {
    pub metadata: MetaData,
    pub body: Option<ContentType>,
    pub headers: HashMap<String, String>,
}
#[derive(Debug)]
pub struct RequestWithState<T: Clone> {
    pub metadata: MetaData,
    pub body: Option<ContentType>,
    pub headers: HashMap<String, String>,
    pub state: T,
}
fn test_handler(req: Request) -> HandlerResponse<'static> {
    Box::pin(async move {
        // This works but isnt really ideal, especially for the user since it is not really clear
        // and straight forward
        let file = std::fs::read_to_string("views/index.html").unwrap();
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("Content-type".to_string(), "text/html".to_string());
        let response: Box<dyn IntoResp + Send> = Box::new((StatusCode::OK, headers, file));
        response
    })
}
fn test_handler_with_state(req: RequestWithState<AppState>) -> HandlerResponse<'static> {
    Box::pin(async move {
        let response: Box<dyn IntoResp + Send> = Box::new((StatusCode::OK, "asdasda".to_string()));
        response
    })
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub hello_page: String,
}
#[tokio::main]
async fn main() -> io::Result<()> {
    let mut new_router = Node::new("/".to_string());
    let new_router = new_router
        .add_handler("/hi/hello".to_string(), test_handler)
        .unwrap()
        .add_handler("/wow/wow".to_string(), test_handler)
        .unwrap()
        .add_handler("/wowo/cool".to_string(), test_handler)
        .unwrap()
        .add_handler("/wow/well".to_string(), test_handler)
        .unwrap()
        .add_handler("/wow".to_string(), test_handler)
        .unwrap();
    dbg!(new_router);
    Ok(())
}
pub async fn handle_conn(
    mut socket: TcpStream,
    handlers: Arc<HashMap<String, HandlerType>>,
    fallback: Option<HandlerType>,
) -> std::io::Result<()> {
    let mut buf = [0; 1024];
    socket.read(&mut buf).await?;
    let req_str = String::from_utf8_lossy(&buf[..]);
    let request = match parse_request(req_str) {
        Ok(request) => request,
        Err(_) => {
            let res = StatusCode::INTERNAL_SERVER_ERROR.into_response();
            socket.write(res.as_bytes()).await?;
            socket.flush().await?;
            return Ok(());
        }
    };

    let handler = match handlers.get(&request.metadata.path) {
        Some(handler) => handler,
        None => match fallback {
            Some(fallback) => {
                let res = fallback(request).await;
                let resp = res.into_response();
                socket.write(resp.as_bytes()).await?;
                socket.flush().await?;
                return Ok(());
            }
            None => {
                let res = StatusCode::NOT_FOUND.into_response();
                socket.write(res.as_bytes()).await?;
                socket.flush().await?;
                return Ok(());
            }
        },
    };
    let res = handler(request).await;
    let response = res.into_response();
    let clone = response.clone();
    socket.write(clone.as_bytes()).await?;
    socket.flush().await?;

    return Ok(());
}
