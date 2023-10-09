pub mod parse;
pub mod request;
pub mod response;
pub mod router;
use crate::request::Request;
use crate::router::Node;
use http::StatusCode;
use request::parse_request;
use response::IntoResp;
use router::HandlerResponse;
use router::MiddlewareResponse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
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
#[derive(Debug, Clone)]
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
pub fn middleware_test(req: Request) -> MiddlewareResponse<'static> {
    Box::pin(async move {
        println!("Hello from middleware {}", req.metadata.path);
        Ok(req)
    })
}

// Might change the request to be called ctx in the future
// since it now holds more that just plain request data.
// Also since state still needs to be added to this struct

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
        let resp = file.replace("{user}", "test_handler");
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("Content-type".to_string(), "text/html".to_string());
        let response: Box<dyn IntoResp + Send> = Box::new((StatusCode::OK, headers, resp));
        response
    })
}

fn test_handler_extract(
    req: Request,
    extract: NewStruct,
    state: AppState,
) -> HandlerResponse<'static> {
    Box::pin(async move { Box::new(StatusCode::OK) as Box<dyn IntoResp + Send> })
}
fn test_handler_user(req: Request) -> HandlerResponse<'static> {
    Box::pin(async move {
        let user = match req.extract {
            Some(user) => user.get("user").unwrap().clone(),
            None => "None".to_string(),
        };
        let returnmsg = "Hello ".to_string() + user.as_str();

        Box::new((StatusCode::OK, returnmsg)) as Box<dyn IntoResp + Send>
    })
}
fn test_handler_user_state(req: Request, state: AppState) -> HandlerResponse<'static> {
    Box::pin(async move {
        Box::new((StatusCode::OK, "hello success".to_string())) as Box<dyn IntoResp + Send>
    })
}
fn test_handler_bytes_state(_req: Request, state: AppState) -> HandlerResponse<'static> {
    Box::pin(async move {
        let mut headers = HashMap::new();
        headers.insert("Content-type".to_string(), "application/wasm".to_string());

        // using the "as" makes this almost usable
        // will still try to implement a solution that abstracts this from the user
        Box::new((StatusCode::OK, headers, state.hello_page)) as Box<dyn IntoResp + Send>
    })
}

#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub hello_page: String,
}
#[derive(Deserialize, Debug, Default, Clone)]
pub struct NewStruct {
    pub hello: String,
}
#[tokio::main]
async fn main() -> io::Result<()> {
    let new_router: Box<Node<AppState>> = Node::new("/".to_string())
        .add_handler("/wow".to_string(), router::Handler::Without(test_handler))
        .unwrap();

    dbg!(&new_router);
    let boxed_router = Box::new(new_router);
    let leaked_router = Box::leak(boxed_router);
    leaked_router.serve("localhost:4000".to_string()).await;
    Ok(())
}
