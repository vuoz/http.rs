pub mod parse;
pub mod request;
pub mod response;
pub mod router;
use crate::request::ContentType;
use crate::request::MetaData;
use crate::request::Request;
use crate::router::Node;
use http::StatusCode;
use request::parse_request;
use response::IntoResp;
use router::HandlerResponse;
use std::collections::HashMap;
use std::io;

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

#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub hello_page: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let new_router: Box<Node<AppState>> = Node::new("/".to_string())
        .add_handler("/wow".to_string(), router::Handler::Without(test_handler))
        .unwrap();
    dbg!(&new_router);
    let router_serveable = new_router.make_into_serveable();
    router_serveable.serve("localhost:4000".to_string()).await;
    Ok(())
}
