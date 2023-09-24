pub mod response;
pub mod router;
use http::StatusCode;
use response::IntoResp;
use router::HandlerResponse;
use router::HandlerType;
use router::Router;
use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::ops::Deref;
use std::pin::Pin;
use std::result::Result;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
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
fn test_handler(req: Request) -> HandlerResponse<'static> {
    Box::pin(async move {
        let response: Box<dyn IntoResp + Send> = Box::new((StatusCode::OK, "asdasda".to_string()));
        response
    })
}
#[tokio::main]
async fn main() -> io::Result<()> {
    let router: Router = Router::new().handle("/dasd", test_handler).await.unwrap();
    router.serve("127.0.0.1:7000".to_string()).await.unwrap();
    Ok(())
}
pub async fn handle_conn(
    mut socket: TcpStream,
    handlers: Arc<HashMap<String, HandlerType>>,
) -> std::io::Result<()> {
    let mut buf = [0; 1024];
    socket.read(&mut buf).await?;
    let req_str = String::from_utf8_lossy(&buf[..]);
    let lines: Vec<&str> = req_str.split("\r\n").collect();
    if lines.len() <= 0 {
        return Err(std::io::Error::new(
            io::ErrorKind::Other,
            "not a valid request",
        ));
    }
    let req_metadata = match parse_method_line(lines.get(0).unwrap()) {
        Some(data) => data,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "cannot parse line",
            ))
        }
    };
    let mut headers: HashMap<String, String> = HashMap::new();
    let mut j = 0;
    for i in 1..lines.len() {
        j += 1;
        let line = match lines.get(i) {
            Some(line) => line,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "cannot parse line",
                ))
            }
        };
        let header = match parse_header(line) {
            Some(h) => h,
            None => break,
        };
        headers.insert(header.key, header.val);
    }

    let body = match lines.get(j + 1) {
        Some(line) => {
            let body_parsed = match parse_body(line) {
                Some(data) => data,
                None => Body::None,
            };
            body_parsed
        }
        None => Body::None,
    };

    let req = match headers.get("content-type") {
        Some(header) => {
            let body = parse_body_new(body, header.clone()).unwrap();
            Request {
                metadata: req_metadata.clone(),
                body: Some(body),
                headers: headers.clone(),
            }
        }
        None => {
            let req = Request {
                metadata: req_metadata.clone(),
                body: None,
                headers,
            };
            req
        }
    };
    let handler = match handlers.get(&req.metadata.path) {
        Some(handler) => handler,
        None => todo!(), //Just return with 404 or use provided fallback handler
    };
    let res = handler(req).await;
    let response = res.into_response();
    let clone = response.clone();
    socket.write(clone.as_bytes()).await?;
    socket.flush().await?;

    return Ok(());
}
pub fn parse_params(inpt: &str) -> Option<ContentType> {
    let params_pairs: Vec<QueryParam> = inpt
        .split("&")
        .map(|param| {
            let params_vec: Vec<String> = param.split("=").map(|param| param.to_string()).collect();
            QueryParam {
                key: params_vec.get(0).unwrap().clone(),
                val: params_vec.get(1).unwrap().clone(),
            }
        })
        .collect();

    let mut queryparams_map = HashMap::new();
    for pair in params_pairs {
        queryparams_map.insert(pair.key, pair.val);
    }
    return Some(ContentType::UrlEncoded(queryparams_map));
}
pub fn parse_body_new(inpt: Body, content_type: String) -> Option<ContentType> {
    match content_type.as_str() {
        "application/x-www-form-urlencoded" => {
            let data = match inpt {
                Body::Binary(_) => return None,
                Body::Text(t) => parse_params(t.as_str()),
                Body::None => return None,
            };
            data
        }
        _ => return None,
    }
}
pub fn parse_body(inpt: &str) -> Option<Body> {
    let parts: Vec<String> = inpt.split("\0").map(|part| part.to_string()).collect();
    let text_part = parts.get(0)?.clone();
    if text_part == "" {
        return None;
    }

    return Some(Body::Text(text_part));
}

pub fn parse_header(inpt: &str) -> Option<Header> {
    let headers: Vec<String> = inpt
        .split(": ")
        .map(|part| part.to_string().to_lowercase())
        .collect();
    if headers.len() != 2 {
        return None;
    }
    let key = headers.get(0)?.clone();
    let val = headers.get(1)?.clone();

    return Some(Header { key, val });
}
pub fn parse_line() -> Option<TypeOfData> {
    return None;
}
pub fn parse_method_line(inpt: &str) -> Option<MetaData> {
    let parts: Vec<String> = inpt.split(" ").map(|part| part.to_string()).collect();
    if parts.len() != 3 {
        return None;
    }
    let method = parts.get(0)?.clone();
    let path = parts.get(1)?.clone();
    let version = parts.get(2)?.clone();
    return Some(MetaData {
        method,
        path,
        version,
    });
}
