#![forbid(unsafe_code)]
use crate::request::Body;
use crate::request::ContentType;
use crate::request::Header;
use crate::request::MetaData;
use crate::request::QueryParam;
use crate::request::TypeOfData;
use std::collections::HashMap;

pub fn parse_params(inpt: &str) -> Option<ContentType> {
    let mut new_map = HashMap::new();
    let _: Vec<()> = inpt
        .split("&")
        .map(|param| {
            let params_vec: Vec<String> = param.split("=").map(|param| param.to_string()).collect();
            if let Some(key) = params_vec.get(0) {
                if let Some(val) = params_vec.get(1) {
                    return Ok(QueryParam {
                        key: key.clone(),
                        val: val.clone(),
                    });
                }
            }
            Err(())
        })
        .take_while(|pair| {
            if let Ok(_) = pair {
                return true;
            }
            return false;
        })
        // this should never panic since we remove any Error Resutls in the prev take_while
        .map(|pair| pair.unwrap())
        .map(|pair| {
            new_map.insert(pair.key, pair.val);
        })
        .collect();
    if new_map.len() == 0 {
        return None;
    }
    dbg!(&new_map);

    return Some(ContentType::UrlEncoded(new_map));
}
pub fn parse_body_new(inpt: Body, content_type: String) -> Option<ContentType> {
    //This implementation will change in the future i do not think this is the correct approach but
    //it works for now
    match content_type.as_str() {
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
