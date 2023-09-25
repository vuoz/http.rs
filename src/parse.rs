use std::collections::HashMap;

use crate::Body;
use crate::ContentType;
use crate::Header;
use crate::MetaData;
use crate::QueryParam;
use crate::TypeOfData;

pub fn parse_params(inpt: &str) -> Option<ContentType> {
    let params_pairs: Vec<QueryParam> = inpt
        .split("&")
        .map(|param| {
            let params_vec: Vec<String> = param.split("=").map(|param| param.to_string()).collect();
            QueryParam {
                key: params_vec.get(0).unwrap().clone(),
                val: params_vec.get(1).unwrap().clone(),
            }
        })
        .collect();

    let mut queryparams_map = HashMap::new();
    for pair in params_pairs {
        queryparams_map.insert(pair.key, pair.val);
    }
    return Some(ContentType::UrlEncoded(queryparams_map));
}
pub fn parse_body_new(inpt: Body, content_type: String) -> Option<ContentType> {
    match content_type.as_str() {
        "application/x-www-form-urlencoded" => {
            let data = match inpt {
                Body::Binary(_) => return None,
                Body::Text(t) => parse_params(t.as_str()),
                Body::None => return None,
            };
            data
        }
        _ => return None,
    }
}
pub fn parse_body(inpt: &str) -> Option<Body> {
    let parts: Vec<String> = inpt.split("\0").map(|part| part.to_string()).collect();
    let text_part = parts.get(0)?.clone();
    if text_part == "" {
        return None;
    }

    return Some(Body::Text(text_part));
}

pub fn parse_header(inpt: &str) -> Option<Header> {
    let headers: Vec<String> = inpt
        .split(": ")
        .map(|part| part.to_string().to_lowercase())
        .collect();
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
    let parts: Vec<String> = inpt.split(" ").map(|part| part.to_string()).collect();
    if parts.len() != 3 {
        return None;
    }
    let method = parts.get(0)?.clone();
    let path = parts.get(1)?.clone();
    let version = parts.get(2)?.clone();
    return Some(MetaData {
        method,
        path,
        version,
    });
}
