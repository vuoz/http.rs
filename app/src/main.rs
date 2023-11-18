#![forbid(unsafe_code)]
use http::StatusCode;
use httpRs::request::Request;
use httpRs::response::respond;
use httpRs::router::HandlerResponse;
use httpRs::router::Redirect;
use httpRs::router::Router;
use serde::Deserialize;
use serde::Serialize;
use std::io;

fn test_handler(_req: Request, _state: AppState) -> HandlerResponse<'static> {
    Box::pin(async move { respond(Redirect::new("/value")) })
}

fn fallback(_req: Request) -> HandlerResponse<'static> {
    Box::pin(async move { respond((StatusCode::NOT_FOUND, "You seem lost".to_string())) })
}

#[derive(Clone, Serialize, Deserialize)]
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
            httpRs::router::Handler::WithState(test_handler),
        )
        .unwrap()
        .with_state(AppState { hello_page: file })
        .fallback(httpRs::router::Handler::Without(fallback))
        .make_into_serveable();
    router.serve("localhost:4000").await;

    Ok(())
}
