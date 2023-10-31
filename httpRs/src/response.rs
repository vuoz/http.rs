#![forbid(unsafe_code)]

use std::collections::HashMap;

use http::StatusCode;

use crate::router::{self, Cookie, Json};

pub trait IntoResp {
    fn into_response(&self) -> Vec<u8>;
}
impl IntoResp for router::Html {
    fn into_response(&self) -> Vec<u8> {
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n{}\r\n{}",
            200,
            "OK",
            self.0.len(),
            "Content-type: text/html".to_owned() + "\r\n",
            self.0,
        );
        return Vec::from(response);
    }
}
impl IntoResp for &str {
    fn into_response(&self) -> Vec<u8> {
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n\r\n{}",
            200,
            "OK",
            self.len(),
            self
        );
        Vec::from(response)
    }
}
impl<T> IntoResp for Json<T>
where
    T: serde::Serialize,
{
    fn into_response(&self) -> Vec<u8> {
        let json_string = match serde_json::to_string(&self.0) {
            Ok(json) => json,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n{}\r\n{}",
            200,
            "OK",
            json_string.len(),
            "Content-type: application/json".to_owned() + "\r\n",
            json_string,
        );
        Vec::from(response)
    }
}
impl IntoResp for (StatusCode, String) {
    fn into_response(&self) -> Vec<u8> {
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n\r\n{}",
            self.0.as_u16(),
            self.0.into_status_message(),
            self.1.len(),
            self.1
        );
        return Vec::from(response);
    }
}
impl IntoResp for StatusCode {
    fn into_response(&self) -> Vec<u8> {
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n\r\n",
            self.as_u16(),
            self.clone().into_status_message(),
            0,
        );
        return Vec::from(response);
    }
}
impl IntoResp for (StatusCode, Vec<u8>) {
    fn into_response(&self) -> Vec<u8> {
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n",
            self.0.as_u16(),
            self.0.into_status_message(),
            self.1.len(),
        );

        let bytes = Vec::from(response);
        let all_bytes = [bytes, self.1.clone()].concat();
        all_bytes
    }
}
impl IntoResp for (StatusCode, HashMap<String, String>, Vec<u8>) {
    fn into_response(&self) -> Vec<u8> {
        let headers_clone = self.1.clone();
        let headers_into_resp: Vec<String> = headers_clone
            .into_iter()
            .map(|(key, val)| format!("{}:{}", key, val))
            .collect();
        let headers_string = headers_into_resp.join("\r\n");
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n{}\r\n",
            self.0.as_u16(),
            self.0.into_status_message(),
            self.2.len(),
            headers_string + "\r\n",
        );

        let bytes = Vec::from(response);
        let all_bytes = [bytes, self.2.clone()].concat();
        all_bytes
    }
}
impl IntoResp for (StatusCode, HashMap<String, String>, String) {
    fn into_response(&self) -> Vec<u8> {
        let headers_clone = self.1.clone();
        let headers_into_resp: Vec<String> = headers_clone
            .into_iter()
            .map(|(key, val)| format!("{}:{}", key, val))
            .collect();
        let headers_string = headers_into_resp.join("\r\n");
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n{}\r\n{}",
            self.0.as_u16(),
            self.0.into_status_message(),
            self.2.len(),
            headers_string + "\r\n",
            self.2
        );
        Vec::from(response)
    }
}
impl IntoResp for (StatusCode, Cookie, String) {
    fn into_response(&self) -> Vec<u8> {
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n{}\r\n{}",
            self.0.as_u16(),
            self.0.into_status_message(),
            self.2.len(),
            self.1.to_header() + "\r\n",
            self.2
        );
        Vec::from(response)
    }
}
trait IntoMessage {
    fn into_status_message(&self) -> String;
}
impl IntoMessage for StatusCode {
    fn into_status_message(&self) -> String {
        self.as_str()
            .replace(&(self.as_u16().to_string().to_owned() + " "), "")
    }
}

pub fn respond(resp: impl IntoResp + std::marker::Send + 'static) -> Box<dyn IntoResp + Send> {
    Box::new(resp)
}
