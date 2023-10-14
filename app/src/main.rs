#![forbid(unsafe_code)]

use http::StatusCode;
use httpRs::request::Request;
use httpRs::response::IntoResp;
use httpRs::router::HandlerResponse;
use httpRs::router::Node;
use std::io;

fn test_handler(req: Request, state: AppState) -> HandlerResponse<'static> {
    Box::pin(async move {
        let mut page = String::new();
        if let Some(extracts) = req.extract {
            page = match extracts.get("page") {
                Some(page) => page.clone(),
                None => return Box::new(StatusCode::BAD_REQUEST) as Box<dyn IntoResp + Send>,
            };
        } else {
            return Box::new(StatusCode::BAD_REQUEST) as Box<dyn IntoResp + Send>;
        }
        // This works but isnt really ideal, especially for the user since it is not really clear
        // and straight forward
        Box::new(httpRs::router::Html(
            state.hello_page.replace("{user}", page.as_str()),
        )) as Box<dyn IntoResp + Send>
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
    router_to_serve.serve("localhost:4000").await;
    Ok(())
}
