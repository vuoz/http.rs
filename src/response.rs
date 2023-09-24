use http::StatusCode;

pub trait IntoResp {
    fn into_response(&self) -> String;
}
impl IntoResp for (StatusCode, String) {
    fn into_response(&self) -> String {
        return "Hello".to_string();
    }
}
