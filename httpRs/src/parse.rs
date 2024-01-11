#![forbid(unsafe_code)]
use crate::types::Method;
use bytes::BytesMut;
use serde::de::DeserializeOwned;

use crate::request::Body;
use crate::request::ContentType;
use crate::request::Header;
use crate::request::MetaData;
use crate::request::ParseError;
use crate::request::QueryParam;

use crate::request::TypeOfData;
use std::borrow::Cow;

use std::collections::HashMap;
// now deprecated
pub fn parse_params(inpt: &str) -> Option<ContentType> {
    let mut new_map = HashMap::new();
    let _: Vec<()> = inpt
        .split("&")
        .map(|param| {
            let params_vec: Vec<String> = param.split("=").map(|param| param.to_string()).collect();
            if let Some(key) = params_vec.get(0) {
                if let Some(val) = params_vec.get(1) {
                    let new_param = QueryParam {
                        key: key.clone(),
                        val: val.clone(),
                    };
                    new_map.insert(new_param.key, new_param.val);
                    ()
                }
            }
            ()
        })
        .collect::<Vec<()>>();
    if new_map.len() == 0 {
        return None;
    }
    return Some(ContentType::UrlEncoded(new_map));
}
// now deprecated
pub fn parse_body_new(inpt: Body, content_type: &str) -> Option<ContentType> {
    //This implementation will change in the future i do not think this is the correct approach but
    //it works for now
    match content_type {
        "application/x-www-form-urlencoded" => {
            let data = match inpt {
                Body::Binary(_) => return None,
                Body::Text(t) => parse_params(t.as_str()),
                Body::None => return None,
            };
            data
        }
        "application/json" => {
            let data = match inpt {
                Body::Binary(_) => return None,
                Body::Text(t) => parse_json(t.as_str()),
                Body::None => return None,
            };
            data
        }

        _ => return None,
    }
}
pub fn parse_json(inpt: &str) -> Option<ContentType> {
    let parts: Vec<String> = inpt.split("\n").map(|part| part.to_string()).collect();
    let text_part = parts.get(0)?.clone();
    if text_part == "" {
        return None;
    }
    return Some(ContentType::Json(text_part));
}
pub fn parse_body(inpt: &str, lenght: u32) -> Option<Body> {
    let mut parts: Vec<String> = inpt.split("\0").map(|part| part.to_string()).collect();
    let text_part = parts.get_mut(0)?;
    if text_part.len() != lenght as usize {
        return None;
    }
    if text_part == "" {
        return None;
    }

    return Some(Body::Text(std::mem::take(text_part)));
}

pub fn parse_header(inpt: &str) -> Option<Header> {
    let headers: Vec<String> = inpt.split(": ").map(|part| part.to_lowercase()).collect();
    if headers.len() != 2 {
        return None;
    }
    let key = headers.get(0)?.clone();
    let val = headers.get(1)?.clone();

    return Some(Header { key, val });
}
pub fn parse_line() -> Option<TypeOfData> {
    return None;
}
pub fn parse_method_line(inpt: &str) -> Option<MetaData> {
    let parts: Vec<&str> = inpt.split(" ").collect();
    if parts.len() != 3 {
        return None;
    }
    let method = parts.get(0)?;
    let path = parts.get(1)?;
    let version = parts.get(2)?;
    return Some(MetaData {
        method: method.to_string(),
        path: path.to_string(),
        version: version.to_string(),
    });
}
#[derive(Clone, Debug, Default)]
pub struct NewRequestType {
    pub metadata: NewMetaData,
    pub body: Option<BytesMut>,
    pub headers: HashMap<String, String>,
    pub params: Option<HashMap<String, String>>,
}
impl NewRequestType {
    pub fn from_json_to_struct<T: DeserializeOwned>(&self) -> std::io::Result<T> {
        match &self.body {
            None => return Err(std::io::Error::from(std::io::ErrorKind::InvalidData)),
            Some(body) => {
                let res: T = match serde_json::from_slice(&body[..]) {
                    Ok(res) => res,
                    Err(e) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string(),
                        ))
                    }
                };
                Ok(res)
            }
        }
    }
}
#[derive(Debug, Default, Clone, PartialEq)]
pub struct NewMetaData {
    pub method: Method,
    pub path: String,
    pub version: String,
}
pub fn parse_new_method_line(line: &str) -> Option<NewMetaData> {
    let mut meta_data = NewMetaData::default();
    let mut count = 0;
    for part in line.split(" ") {
        count += 1;
        if count == 1 {
            meta_data.method = match Method::from_bytes(part.as_bytes()) {
                Ok(res) => res,
                Err(_) => return None,
            };
            continue;
        }
        if count == 2 {
            meta_data.path = part.to_string();
            continue;
        }
        if count == 3 {
            meta_data.version = part.to_string();
            continue;
        }
    }
    if count != 3 {
        return None;
    }
    return Some(meta_data);
}
pub fn parse_header_new(line: &str) -> Option<(&str, &str)> {
    let mut key = None;
    let mut val = None;
    for (count, part) in line.split(": ").enumerate() {
        if count == 0 {
            key = Some(part);
            continue;
        }
        if count == 1 {
            val = Some(part);
            continue;
        } else {
            return None;
        }
    }
    match key {
        Some(key) => match val {
            Some(val) => return Some((key, val)),
            None => return None,
        },
        None => return None,
    };
}
pub fn parse_params_from_path(path_after_question_mark: &str) -> Option<HashMap<String, String>> {
    let mut map: Option<HashMap<String, String>> = None;
    'outer: for param_pair in path_after_question_mark.split("&") {
        match map.as_mut() {
            Some(map) => {
                let mut key = None;
                let mut val = None;
                'inner: for (count, part) in param_pair.split("=").enumerate() {
                    if count == 0 {
                        key = Some(part);
                        continue 'inner;
                    }
                    if count == 1 {
                        val = Some(part);
                        continue 'inner;
                    } else {
                        continue 'outer;
                    }
                }
                match key {
                    Some(key) => match val {
                        Some(val) => {
                            map.insert(key.to_string(), val.to_string());
                        }
                        None => continue 'outer,
                    },
                    None => continue 'outer,
                }
            }
            None => {
                let mut key = None;
                let mut val = None;
                'inner: for (count, part) in param_pair.split("=").enumerate() {
                    if count == 0 {
                        key = Some(part);
                        continue 'inner;
                    }
                    if count == 1 {
                        val = Some(part);
                        continue 'inner;
                    } else {
                        continue 'outer;
                    }
                }
                match key {
                    Some(key) => match val {
                        Some(val) => {
                            let mut map_local = HashMap::new();
                            map_local.insert(key.to_string(), val.to_string());
                            map = Some(map_local)
                        }
                        None => continue 'outer,
                    },
                    None => continue 'outer,
                }
            }
        }
    }
    return map;
}

pub fn parse_request(req_str: &Cow<'_, str>) -> Result<NewRequestType, ParseError> {
    let mut request = NewRequestType::default();
    let mut header_before = false;
    let mut count = 0;
    let req_string = req_str.clone();
    let mut lines: Vec<&str> = req_string.split("\r\n").collect();
    for line in &lines {
        count += 1;
        if count == 1 {
            match parse_new_method_line(&line) {
                Some(mut parse_res) => {
                    if parse_res.path.contains("?") {
                        let split_res = parse_res.path.split_once("?").unwrap();
                        match parse_params_from_path(split_res.1) {
                            Some(params) => request.params = Some(params),
                            None => (),
                        };
                        parse_res.path = split_res.0.to_string();
                    }
                    request.metadata = parse_res;
                }

                None => return Err(ParseError::NotValidRequest),
            }
            continue;
        }
        if let Some(header) = parse_header_new(line) {
            request
                .headers
                .insert(header.0.to_string(), header.0.to_string());
            header_before = true;
            continue;
        }
        if !header_before {
            break;
        }
        header_before = false;
    }

    if &lines.len() == &0 {
        request.body = None
    } else {
        let body = lines.swap_remove(&lines.len() - 1 as usize);
        if body == "" {
            request.body = None;
        } else {
            request.body = Some(BytesMut::from(body))
        }
    }

    return Ok(request.clone());
}
