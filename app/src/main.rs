#![forbid(unsafe_code)]
use http::StatusCode;
use httpRs::parse::NewRequestType;
use httpRs::response::respond;
use httpRs::router::HandlerResponse;
use httpRs::router::Html;
use httpRs::router::Router;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::io;

fn test_handler(
    _req: NewRequestType,
    state: AppState,
    _extract: HashMap<String, String>,
) -> HandlerResponse<'static> {
    Box::pin(async move {
        let json_body: JsonTest = _req.from_json_to_struct().unwrap();
        println!("{:?}", json_body);
        respond(Html(state.hello_page))
    })
}

fn fallback(_req: NewRequestType) -> HandlerResponse<'static> {
    Box::pin(async move { respond((StatusCode::NOT_FOUND, "You seem lost".to_string())) })
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct JsonTest {
    pub test_string: String,
    pub page: String,
}

#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub hello_page: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let file = std::fs::read_to_string("views/index.html").unwrap();
    let router = Router::new()
        .add_handler(
            "/wow/:user",
            httpRs::router::Handler::WithStateAndExtract(test_handler),
        )
        .unwrap()
        .with_state(AppState { hello_page: file })
        .fallback(httpRs::router::Handler::Without(fallback))
        .make_into_serveable();
    router.serve("localhost:4000").await;

    Ok(())
}
