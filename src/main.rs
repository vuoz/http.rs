use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
#[derive(Debug)]
pub enum HandleConnError {
    Error(std::io::Error),
}
impl From<std::io::Error> for HandleConnError {
    fn from(value: std::io::Error) -> Self {
        HandleConnError::Error(value)
    }
}

fn main() {
    println!("Hello, world!");
    let listener = match TcpListener::bind("127.0.0.1:7000") {
        Ok(l) => l,
        Err(e) => {
            println!("{e}");
            return;
        }
    };
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                println!("{e}");
                return;
            }
        };
        match handle_conn(stream) {
            Ok(_) => (),
            Err(e) => {
                println!("{:?}", e);
                return;
            }
        };
    }
}
pub fn handle_conn(mut stream: TcpStream) -> Result<(), HandleConnError> {
    let mut prebuf = [0; 1024];
    stream.read(&mut prebuf)?;
    println!("Request {}", String::from_utf8_lossy(&prebuf[..]));
    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write(response.as_bytes())?;

    stream.flush()?;
    stream.shutdown(std::net::Shutdown::Both)?;
    return Ok(());
}
