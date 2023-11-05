#![forbid(unsafe_code)]
pub mod h2;
pub mod parse;
pub mod request;
pub mod response;
pub mod router;
pub mod tls;
#[cfg(test)]
mod tests {
    use http::StatusCode;

    use crate::{request::*, router};

    use crate::parse::parse_method_line;
    use crate::response::respond;
    use crate::router::{HandlerResponse, Node};

    #[test]
    fn parse() {
        let test_lines = vec![
            (
                "PUT / HTTP/1",
                MetaData {
                    method: String::from("PUT"),
                    path: String::from("/"),
                    version: String::from("HTTP/1"),
                },
            ),
            (
                "POST /value HTTP/1",
                MetaData {
                    method: String::from("POST"),
                    path: String::from("/value"),
                    version: String::from("HTTP/1"),
                },
            ),
            (
                "GET /value/path HTTP/1",
                MetaData {
                    method: String::from("GET"),
                    path: String::from("/value/path"),
                    version: String::from("HTTP/1"),
                },
            ),
        ];
        for i in test_lines.into_iter() {
            let parse_res = match parse_method_line(i.0) {
                Some(res) => res,
                None => panic!("Test failed"),
            };
            assert_eq!(parse_res, i.1)
        }
    }
    #[test]
    fn route() {
        fn test_fn(_req: Request) -> HandlerResponse<'static> {
            Box::pin(async move { respond(StatusCode::OK) })
        }
        let node = Node::<()>::new("/")
            .add_handler("/test", router::Handler::Without(test_fn))
            .unwrap()
            .add_handler("/value/:user/wow/:ts", router::Handler::Without(test_fn))
            .unwrap()
            .add_handler("/test/:user", router::Handler::Without(test_fn))
            .unwrap();
        match node.get_handler(String::from("/test")) {
            None => panic!("test failure"),
            Some(_) => (),
        }
        match node.get_handler(String::from("/value/user1/wow/1235")) {
            None => panic!("test failure"),
            Some(_) => (),
        }
        match node.get_handler(String::from("/test/user1")) {
            None => panic!("test failure"),
            Some(_) => (),
        }
    }
}
