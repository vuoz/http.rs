#![forbid(unsafe_code)]
use bytes::Bytes;
use h2::{server::SendResponse, *};
use http::Request;
use std::error::Error;
use tokio::net::TcpStream;

pub async fn handle_h2(socket: TcpStream) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut connection = server::handshake(socket).await?;
    while let Some(result) = connection.accept().await {
        let (request, respond) = result?;
        tokio::spawn(async move {
            match handle_req_h2(request, respond).await {
                Ok(_) => (),
                Err(e) => panic!("Error handling http2 request {e}"),
            };
        });
    }
    Ok(())
}
async fn handle_req_h2(
    request: Request<RecvStream>,
    mut respond: SendResponse<Bytes>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let body = request.body();
    let response = http::Response::new(());
    let mut send = respond.send_response(response, false)?;
    send.send_data(Bytes::from_static(b"hello"), true)?;
    Ok(())
}
