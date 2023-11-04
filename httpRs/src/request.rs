#![forbid(unsafe_code)]

use serde::de::DeserializeOwned;

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

#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug)]
pub enum ParseError {
    Empty,
    NotValidRequest,
    CannotParseMetaData,
}
#[derive(Debug, Clone)]
pub struct Request {
    pub metadata: MetaData,
    //pub extract: Option<HashMap<String, String>>,
    pub body: Option<ContentType>,
    pub headers: HashMap<String, String>,
    //pub cookies: Option<HashMap<String, String>>,
}
impl Request {
    pub fn from_json_to_struct<T: DeserializeOwned>(self) -> std::io::Result<T> {
        let body = match self.body {
            None => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "no body")),
            Some(body) => body,
        };
        let json_string = match body {
            ContentType::Json(data) => data,
            ContentType::Binary(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "wrong type of data",
                ))
            }
            ContentType::PlainText(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "wrong type of data",
                ))
            }
            ContentType::UrlEncoded(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "wrong type of data",
                ))
            }
            ContentType::None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "wrong type of data",
                ))
            }
        };
        let data: T = serde_json::from_str(&json_string)?;
        Ok(data)
    }
    pub fn cookies(&self) -> Option<HashMap<String, String>> {
        match &self.headers.get("cookie") {
            Some(cookies) => {
                let cookies_separated: Vec<&str> = cookies.split(";").collect();
                let mut cookie_map = HashMap::new();
                let _: Vec<&str> = cookies_separated
                    .into_iter()
                    .take_while(|cookie| {
                        let parts: Vec<&str> = cookie.split("=").collect();
                        if parts.len() != 2 {
                            return false;
                        }
                        let mut name = match parts.get(0) {
                            Some(name) => name.to_string(),
                            None => return false,
                        };
                        if name.contains(" ") {
                            name = name.replace(" ", "");
                        }
                        let mut value = match parts.get(1) {
                            Some(name) => name.to_string(),
                            None => return false,
                        };
                        if value.contains(" ") {
                            value = value.replace(" ", "");
                        }
                        cookie_map.insert(name, value);
                        return true;
                    })
                    .collect();

                return Some(cookie_map);
            }
            None => return None,
        }
    }
}
#[derive(Debug, Clone)]
pub struct ParseRes {
    pub metadata: MetaData,
    pub extract: Option<HashMap<String, String>>,
    pub body: Option<ContentType>,
    pub headers: HashMap<String, String>,
}
pub trait ToRequest {
    fn to_request(self) -> Request;
}
impl ToRequest for ParseRes {
    fn to_request(self) -> Request {
        Request {
            metadata: self.metadata,
            body: self.body,
            headers: self.headers,
        }
    }
}

pub fn parse_request(req_str: Cow<'_, str>) -> Result<ParseRes, ParseError> {
    let lines: Vec<&str> = req_str.split("\r\n").collect();
    dbg!(&lines);
    if lines.len() <= 0 {
        return Err(ParseError::NotValidRequest);
    }
    let method_line = match lines.get(0) {
        Some(line) => line,
        None => return Err(ParseError::Empty),
    };
    let req_metadata = match parse_method_line(&method_line) {
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

    let body;
    if req_metadata.method == "POST" || req_metadata.method == "PUT" {
        let content_lenght = match headers.get("content-length") {
            Some(l) => match l.parse() {
                Err(_) => return Err(ParseError::NotValidRequest),
                Ok(l) => l,
            },
            None => return Err(ParseError::NotValidRequest),
        };

        body = match lines.get(j + 1) {
            Some(line) => {
                let body_parsed = match parse_body(line, content_lenght) {
                    Some(data) => data,
                    None => Body::None,
                };
                body_parsed
            }
            None => Body::None,
        };
    } else {
        body = Body::None;
    }
    let mut req = match headers.get("content-type") {
        Some(header) => {
            let body = parse_body_new(body, header).unwrap();
            ParseRes {
                metadata: req_metadata,
                body: Some(body),
                headers,
                extract: None,
            }
        }
        None => {
            let req = ParseRes {
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
