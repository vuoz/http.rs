use std::collections::HashMap;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::ops::Deref;
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
pub struct Header {
    pub key: String,
    pub val: String,
}
pub enum Body {
    Binary(Vec<u8>),
    Text(String),
    MetaData(MetaData),
}

pub enum TypeOfData {
    Header(Header),
    Body(Body),
}

#[derive(Debug)]
pub struct MetaData {
    pub method: String,
    pub path: String,
    pub version: String,
}
// this will prob change but the
// idea is to have a trait that all handlers must impl
// so that different functions can be used to handle
/*
trait handlable {
    fn handle(&self) -> http::StatusCode;
}*/
// Will probably need a map for all the routes registered
// But this will probably be in a struct that will be impled
// like this
// struct Router{
//    routes: Arc<HashMap<String,handlable>>,
// }
// impl Router{}
// ...
/*
let map: HashMap<String, String /*placeholder*/> = HashMap::new();
let routermap = Arc::new(map);
*/
#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7000").await?;

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            match handle_conn(socket).await {
                Ok(_) => (),
                Err(e) => {
                    println!("{e}");
                    return;
                }
            };
        });
    }
}
pub async fn handle_conn(mut socket: TcpStream) -> std::io::Result<()> {
    let mut buf = [0; 1024];
    socket.read(&mut buf).await?;
    let req_str = String::from_utf8_lossy(&buf[..]);
    let lines: Vec<&str> = req_str.split("\r\n").collect();
    println!("Request {:#?}", lines);
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
    dbg!(req_metadata);
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
    // lines.get(j+1);
    //then parse body
    //Then give all the data to the handler after getting the correct handler with the path

    /*
    let file = "Hello file";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Lenght: {}\r\n\r\n{}",
        file.len(),
        file
    );
    socket.write(response.as_bytes()).await?;
    socket.flush().await?;*/
    return Ok(());
}
pub fn parse_body(inpt: &str) -> Option<Body> {
    return None;
}

pub fn parse_header(inpt: &str) -> Option<Header> {
    let headers: Vec<String> = inpt.split(":").map(|part| part.to_string()).collect();
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
