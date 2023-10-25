#![forbid(unsafe_code)]
use httpRs::request::Request;
use httpRs::response::IntoResp;
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
    extracts: HashMap<String, String>,
) -> HandlerResponse<'static> {
    Box::pin(async move {
        //let data: JsonTest = req.from_json_to_struct().unwrap();
        // This works but isnt really ideal, especially for the user since it is not really clear
        // and straight forward
        let resp_obj = JsonTest {
            test_string: String::from("wow"),
            page: String::from("wow"),
        };
        dbg!(extracts);
        Box::new(Json(resp_obj)) as Box<dyn IntoResp + Send>
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
