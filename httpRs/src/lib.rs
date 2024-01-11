#![forbid(unsafe_code)]
pub mod h2;
pub mod parse;
pub mod request;
pub mod response;
pub mod router;
pub mod tls;
pub mod types;
#[cfg(test)]
mod tests {

    use crate::parse::NewMetaData;
    use crate::types::Method;

    use crate::parse::parse_new_method_line;

    #[test]
    fn parse() {
        let test_lines = vec![
            (
                "PUT / HTTP/1",
                NewMetaData {
                    method: Method::PUT,
                    path: String::from("/"),
                    version: String::from("HTTP/1"),
                },
            ),
            (
                "POST /value HTTP/1",
                NewMetaData {
                    method: Method::POST,
                    path: String::from("/value"),
                    version: String::from("HTTP/1"),
                },
            ),
            (
                "GET /value/path HTTP/1",
                NewMetaData {
                    method: Method::GET,
                    path: String::from("/value/path"),
                    version: String::from("HTTP/1"),
                },
            ),
        ];
        for i in test_lines.into_iter() {
            let parse_res = match parse_new_method_line(i.0) {
                Some(res) => res,
                None => panic!("Test failed"),
            };
            assert_eq!(parse_res, i.1)
        }
    }
}
