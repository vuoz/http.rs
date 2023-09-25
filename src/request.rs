use crate::parse_body;
use crate::parse_body_new;
use crate::parse_header;
use crate::parse_method_line;
use crate::Body;
use std::borrow::Cow;
use std::collections::HashMap;

use crate::Request;
pub enum ParseError {
    Empty,
    NotValidRequest,
    CannotParseMetaData,
}

pub fn parse_request(req_str: Cow<'_, str>) -> Result<Request, ParseError> {
    let lines: Vec<&str> = req_str.split("\r\n").collect();
    if lines.len() <= 0 {
        return Err(ParseError::NotValidRequest);
    }
    let req_metadata = match parse_method_line(lines.get(0).unwrap()) {
        Some(data) => data,
        None => return Err(ParseError::CannotParseMetaData),
    };
    let mut headers: HashMap<String, String> = HashMap::new();
    let mut j = 0;
    for i in 1..lines.len() {
        j += 1;
        let line = match lines.get(i) {
            Some(line) => line,
            None => return Err(ParseError::CannotParseMetaData),
        };
        let header = match parse_header(line) {
            Some(h) => h,
            None => break,
        };
        headers.insert(header.key, header.val);
    }

    let body = match lines.get(j + 1) {
        Some(line) => {
            let body_parsed = match parse_body(line) {
                Some(data) => data,
                None => Body::None,
            };
            body_parsed
        }
        None => Body::None,
    };

    let req = match headers.get("content-type") {
        Some(header) => {
            let body = parse_body_new(body, header.clone()).unwrap();
            Request {
                metadata: req_metadata.clone(),
                body: Some(body),
                headers: headers.clone(),
            }
        }
        None => {
            let req = Request {
                metadata: req_metadata.clone(),
                body: None,
                headers,
            };
            req
        }
    };
    return Ok(req);
}
