use std::io;
use std::io::prelude::*;
use std::result::Result;
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
    println!("{}", String::from_utf8_lossy(&buf[..]));
    let file = std::fs::read_to_string("views/index.html").unwrap();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Lenght: {}\r\n\r\n{}",
        file.len(),
        file
    );
    socket.write(response.as_bytes()).await?;
    socket.flush().await?;
    return Ok(());
}
