use crate::request::*;
use crate::response::IntoResp;
use crate::router::HandlerType;
use crate::router::Node;
use crate::router::RoutingResult;
use http::StatusCode;
use rustls::Certificate;
use rustls::PrivateKey;
use std::fs::File;
use std::io::BufReader;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio_rustls::TlsStream;

pub fn load_certificates_from_pem(path: &str) -> std::io::Result<Vec<Certificate>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader)?;

    Ok(certs.into_iter().map(Certificate).collect())
}

pub fn load_private_key_from_file(path: &str) -> Result<PrivateKey, Box<dyn std::error::Error>> {
    let file = File::open(&path)?;
    let mut reader = BufReader::new(file);
    let mut keys = rustls_pemfile::pkcs8_private_keys(&mut reader)?;

    match keys.len() {
        0 => Err(format!("No PKCS8-encoded private key found in {path}").into()),
        1 => Ok(PrivateKey(keys.remove(0))),
        _ => Err(format!("More than one PKCS8-encoded private key found in {path}").into()),
    }
}
pub async fn handle_conn_node_based_tls<
    T: std::clone::Clone
        + std::default::Default
        + std::marker::Send
        + std::marker::Sync
        + std::fmt::Debug,
>(
    mut socket: TlsStream<tokio::net::TcpStream>,
    handlers: &Node<T>,
    fallback: Option<HandlerType>,
    state: Option<T>,
) -> std::io::Result<()> {
    let mut buf = Vec::with_capacity(1024);
    socket.read_buf(&mut buf).await?;
    let req_str = String::from_utf8_lossy(&buf[..]);
    let res = crate::parse::parse_request(&req_str).unwrap();

    let routing_res: RoutingResult<T> = match handlers.get_handler(res.metadata.path.clone()) {
        Some(res) => res,
        None => match fallback {
            Some(fallback) => {
                let res = fallback(res).await;
                let resp = res.into_response();
                socket.write_all(resp.as_slice()).await?;
                socket.flush().await?;
                socket.shutdown().await?;
                return Ok(());
            }
            None => {
                send_error_response_tls(socket, StatusCode::NOT_FOUND).await?;
                return Ok(());
            }
        },
    };
    let handler = routing_res.handler;

    // will try to find another solution other to cloning this map
    let map_clone = res.params.clone();
    let res = match handler
        .handle(
            res,
            state,
            // This is needed since there are two ways extracts can be added to the request
            // The first being for example /user/:id which comes from the router
            // And the second being on the end of the request path for example
            // /user/:id?page=10
            // This gets parsed by the request parser so we need to merge the two maps
            match routing_res.extract {
                Some(mut res) => match map_clone {
                    Some(map2) => {
                        res.extend(map2.into_iter());
                        Some(res)
                    }

                    None => Some(res),
                },
                None => None,
            },
        )
        .await
    {
        Some(res) => res,
        None => return send_error_response_tls(socket, StatusCode::NOT_FOUND).await,
    };
    let response = res.into_response();
    let clone = response.clone();
    socket.write_all(clone.as_slice()).await?;
    socket.flush().await?;
    socket.shutdown().await?;
    Ok(())
}
async fn send_error_response_tls(
    mut socket: TlsStream<tokio::net::TcpStream>,
    code: StatusCode,
) -> std::io::Result<()> {
    let res = code.into_response();
    socket.write_all(res.as_slice()).await?;
    socket.flush().await?;
    socket.shutdown().await?;
    Ok(())
}
