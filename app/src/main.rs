#![forbid(unsafe_code)]

use std::collections::HashMap;
use httpRs::response::IntoResp;
use http::StatusCode;
use httpRs::request::Request;
use httpRs::router::HandlerResponse;
use httpRs::router::Node;
use std::io;



fn test_handler(req: Request, state: AppState) -> HandlerResponse<'static> {
    Box::pin(async move {
        dbg!(&req.extract);
        // This works but isnt really ideal, especially for the user since it is not really clear
        // and straight forward
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("Content-type".to_string(), "text/html".to_string());
        let response: Box<dyn IntoResp + Send> =
            Box::new((StatusCode::OK, headers, state.hello_page));
        response
    })
}

#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub hello_page: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let file = std::fs::read_to_string("views/index.html").unwrap();
    let router = Node::new("/")
        .add_handler("/wow", httpRs::router::Handler::WithState(test_handler))
        .unwrap()
        .add_state(AppState { hello_page: file });
    dbg!(&router);
    let router_to_serve = router.make_into_serveable();
    router_to_serve.serve("localhost:4000".to_string()).await;
    Ok(())
}
