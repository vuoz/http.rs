#![forbid(unsafe_code)]

use crate::parse::parse_body;
use crate::parse::parse_body_new;
use crate::parse::parse_header;
use crate::parse::parse_method_line;
use crate::parse::parse_params;

use std::borrow::Cow;
use std::collections::HashMap;
#[derive(Debug)]
pub struct Header {
    pub key: String,
    pub val: String,
}
#[derive(Debug)]
pub enum Body {
    Binary(Vec<u8>),
    Text(String),
    None,
}
#[derive(Debug, Clone)]
pub enum ContentType {
    Json(String),
    UrlEncoded(HashMap<String, String>),
    PlainText(String),
    Binary(Vec<u8>),
    None,
}
#[derive(Debug, Clone)]
pub struct QueryParam {
    pub key: String,
    pub val: String,
}

pub enum TypeOfData {
    Header(Header),
    Body(Body),
}

#[derive(Debug, Clone)]
pub struct MetaData {
    pub method: String,
    pub path: String,
    pub version: String,
}

#[derive(Clone, Debug)]
pub struct RouteExtract {
    pub identifier: String,
    pub value: String,
}
pub enum ParseError {
    Empty,
    NotValidRequest,
    CannotParseMetaData,
}
#[derive(Debug, Clone)]
pub struct Request {
    pub metadata: MetaData,
    pub extract: Option<HashMap<String, String>>,
    pub body: Option<ContentType>,
    pub headers: HashMap<String, String>,
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

    let mut req = match headers.get("content-type") {
        Some(header) => {
            let body = parse_body_new(body, header).unwrap();
            Request {
                metadata: req_metadata,
                body: Some(body),
                headers,
                extract: None,
            }
        }
        None => {
            let req = Request {
                metadata: req_metadata,
                body: None,
                headers,
                extract: None,
            };
            req
        }
    };
    // This parses the requests query params even if the correct content-type is not set
    // So even a get request without the header can be /page?id=3 an still me matched correctly
    if req.metadata.path.contains("?") {
        let splits: Vec<String> = req
            .metadata
            .path
            .split("?")
            .map(|split| split.to_string())
            .collect();
        if splits.len() != 2 {
            return Err(ParseError::NotValidRequest);
        }
        if let Some(path) = splits.get(0) {
            req.metadata.path = path.clone();
        } else {
            return Err(ParseError::NotValidRequest);
        }
        if let Some(params) = splits.get(1) {
            let extract_result = parse_params(params);
            let extract = match extract_result {
                None => return Err(ParseError::NotValidRequest),
                // This match isn't really ideal will try to find another solution
                Some(result) => {
                    let res = match result {
                        crate::request::ContentType::Json(_) => {
                            return Err(ParseError::NotValidRequest)
                        }
                        crate::request::ContentType::UrlEncoded(res) => res,
                        crate::request::ContentType::PlainText(_) => {
                            return Err(ParseError::NotValidRequest)
                        }
                        crate::request::ContentType::Binary(_) => {
                            return Err(ParseError::NotValidRequest)
                        }
                        crate::request::ContentType::None => {
                            return Err(ParseError::NotValidRequest)
                        }
                    };

                    res
                }
            };
            req.extract = Some(extract);
        } else {
            return Err(ParseError::NotValidRequest);
        }
    }

    return Ok(req);
}
