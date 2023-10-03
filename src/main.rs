pub mod parse;
pub mod request;
pub mod response;
pub mod router;
use crate::router::Node;
use http::StatusCode;
use request::parse_request;
use request::RouteExtract;
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
// Might change the request to be called ctx in the future
// since it now holds more that just plain request data.
// Also since state still needs to be added to this struct
#[derive(Debug, Clone)]
pub struct Request {
    pub metadata: MetaData,
    // Could also make the Extract a HashMap
    pub extract: Option<HashMap<String, String>>,
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
        dbg!(req.extract);
        // This works but isnt really ideal, especially for the user since it is not really clear
        // and straight forward
        let file = std::fs::read_to_string("views/index.html").unwrap();
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("Content-type".to_string(), "text/html".to_string());
        let response: Box<dyn IntoResp + Send> = Box::new((StatusCode::OK, headers, file));
        response
    })
}
fn test_handler_user(req: Request) -> HandlerResponse<'static> {
    Box::pin(async move {
        let user = match req.extract {
            Some(user) => user.get("user").unwrap().clone(),
            None => "None".to_string(),
        };
        let returnmsg = "Hello ".to_string() + user.as_str();
        // using the "as" makes this almost usable :()
        // will still try to implement a solution that abstracts this from the user
        Box::new((StatusCode::OK, returnmsg)) as Box<dyn IntoResp + Send>
    })
}
fn test_handler_user_state(req: Request, state: AppState) -> HandlerResponse<'static> {
    Box::pin(async move {
        let user = match req.extract {
            Some(user) => user.get("user").unwrap().clone(),
            None => "None".to_string(),
        };
        let returnmsg = state.hello_page;
        let res = returnmsg.replace("{user}", &user);
        let mut headers = HashMap::new();
        headers.insert("Content-type".to_string(), "text/html".to_string());

        // using the "as" makes this almost usable :()
        // will still try to implement a solution that abstracts this from the user
        Box::new((StatusCode::OK, headers, res)) as Box<dyn IntoResp + Send>
    })
}
fn test_handler_bytes_state(_req: Request, state: AppState) -> HandlerResponse<'static> {
    Box::pin(async move {
        let mut headers = HashMap::new();
        headers.insert("Content-type".to_string(), "application/wasm".to_string());

        // using the "as" makes this almost usable :()
        // will still try to implement a solution that abstracts this from the user
        Box::new((StatusCode::OK, headers, state.hello_page)) as Box<dyn IntoResp + Send>
    })
}

#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub hello_page: String,
    //pub wasm: Vec<u8>,
}
#[tokio::main]
async fn main() -> io::Result<()> {
    let file = std::fs::read_to_string("views/index.html").unwrap();
    //let wasm = std::fs::read("main.wasm").unwrap();
    let app_state = AppState {
        hello_page: file,
        //wasm,
    };
    let mut new_router: Node<AppState> = Node::new("/".to_string());
    let new_router_2 = new_router
        .add_handler(
            "/over/:user".to_string(),
            router::Handler::WithState(test_handler_user_state),
        )
        .unwrap()
        .add_handler(
            "/cool/wow".to_string(),
            router::Handler::WithState(test_handler_bytes_state),
        )
        .unwrap()
        .add_handler(
            "/user/:id/time/:ts".to_string(),
            router::Handler::Without(test_handler),
        )
        .unwrap()
        .add_state(app_state);
    dbg!(&new_router_2);
    let boxed_router = Box::new(new_router_2);
    let leaked_router = Box::leak(boxed_router);
    leaked_router.serve("localhost:4000".to_string()).await;
    Ok(())
}

pub async fn handle_conn_node_based<
    T: std::clone::Clone + std::default::Default + std::marker::Send + std::marker::Sync,
>(
    mut socket: TcpStream,
    handlers: &Node<T>,
    fallback: Option<HandlerType>,
    state: Option<T>,
) -> std::io::Result<()> {
    let mut buf = [0; 1024];
    socket.read(&mut buf).await?;
    let req_str = String::from_utf8_lossy(&buf[..]);
    let mut request = match parse_request(req_str) {
        Ok(request) => request,
        Err(_) => {
            let res = StatusCode::INTERNAL_SERVER_ERROR.into_response();
            socket.write(res.as_slice()).await?;
            socket.flush().await?;
            return Ok(());
        }
    };

    let routing_res = match handlers.get_handler(request.metadata.path.clone()) {
        Some(res) => res,
        None => match fallback {
            Some(fallback) => {
                let res = fallback(request).await;
                let resp = res.into_response();
                socket.write(resp.as_slice()).await?;
                socket.flush().await?;
                return Ok(());
            }
            None => {
                let res = StatusCode::NOT_FOUND.into_response();
                socket.write(res.as_slice()).await?;
                socket.flush().await?;
                return Ok(());
            }
        },
    };
    let handler = routing_res.handler;
    if let Some(extract) = routing_res.extract {
        request.extract = Some(extract);
    }
    let res = match handler.handle(request, state).await {
        Some(res) => res,
        None => {
            let res = StatusCode::NOT_FOUND.into_response();
            socket.write(res.as_slice()).await?;
            socket.flush().await?;
            return Ok(());
        }
    };
    let response = res.into_response();
    let clone = response.clone();
    socket.write(clone.as_slice()).await?;
    socket.flush().await?;

    return Ok(());
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
            socket.write(res.as_slice()).await?;
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
                socket.write(resp.as_slice()).await?;
                socket.flush().await?;
                return Ok(());
            }
            None => {
                let res = StatusCode::NOT_FOUND.into_response();
                socket.write(res.as_slice()).await?;
                socket.flush().await?;
                return Ok(());
            }
        },
    };
    let res = handler(request).await;
    let response = res.into_response();
    let clone = response.clone();
    socket.write(clone.as_slice()).await?;
    socket.flush().await?;

    return Ok(());
}
