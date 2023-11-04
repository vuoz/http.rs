#![forbid(unsafe_code)]
pub mod h2;
pub mod parse;
pub mod request;
pub mod response;
pub mod router;
pub mod tls;
#[cfg(test)]
mod tests {
    use crate::request::*;

    use crate::parse::parse_method_line;

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
}
