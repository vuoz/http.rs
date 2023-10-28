#![forbid(unsafe_code)]

use httpRs::request::Request;
use httpRs::response::respond;
use httpRs::router::HandlerResponse;
use httpRs::router::Json;
use httpRs::router::Node;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

use std::io;

fn test_handler(
    req: Request,
    _state: AppState,
    _extracts: HashMap<String, String>,
) -> HandlerResponse<'static> {
    Box::pin(async move {
        let data: JsonTest = req.from_json_to_struct().unwrap();
        let resp_obj = JsonTest {
            test_string: data.test_string,
            page: data.page,
        };
        respond(Json(resp_obj))
    })
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
    let router = Node::new("/")
        .add_handler(
            "/wow/:user",
            httpRs::router::Handler::WithStateAndExtract(test_handler),
        )
        .unwrap()
        .add_state(AppState { hello_page: file });
    dbg!(&router);
    let router_to_serve = router.make_into_serveable();
    router_to_serve.serve("localhost:4000").await;
    Ok(())
}
