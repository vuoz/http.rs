use std::collections::HashMap;

use http::StatusCode;

pub trait IntoResp {
    fn into_response(&self) -> String;
}
impl IntoResp for (StatusCode, String) {
    fn into_response(&self) -> String {
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n\r\n{}",
            self.0.as_u16(),
            status_to_string(self.0),
            self.1.len(),
            self.1
        );
        return response;
    }
}
impl IntoResp for StatusCode {
    fn into_response(&self) -> String {
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n\r\n",
            self.as_u16(),
            status_to_string(self.clone()),
            0,
        );
        return response;
    }
}
impl IntoResp for (StatusCode, HashMap<String, String>, String) {
    fn into_response(&self) -> String {
        let headers_clone = self.1.clone();
        let headers_into_resp: Vec<String> = headers_clone
            .into_iter()
            .map(|(key, val)| format!("{}:{}", key, val))
            .collect();
        let headers_string = headers_into_resp.join("\r\n");
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n{}\r\n{}",
            self.0.as_u16(),
            status_to_string(self.0),
            self.2.len(),
            headers_string + "\r\n",
            self.2
        );
        response
    }
}

// Would rather do it with a trait but this is a quick solution
// Will change that in the future
pub fn status_to_string(code: StatusCode) -> String {
    match code {
        StatusCode::OK => "Ok".to_string(),
        StatusCode::NOT_FOUND => "NOT FOUND".to_string(),
        StatusCode::FORBIDDEN => "FORBIDDEN".to_string(),
        StatusCode::UNPROCESSABLE_ENTITY => "NOT PROCESSABLE".to_string(),
        _ => "INTERNAL SERVER ERROR".to_string(),
    }
}
