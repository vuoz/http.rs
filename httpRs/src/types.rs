#[derive(Clone, Debug, Default, PartialEq)]
pub enum Method {
    #[default]
    GET,
    PUT,
    POST,
    HEAD,
    PATCH,
    TRACE,
    DELETE,
    OPTIONS,
    CONNECT,
}
pub enum MethodError {
    Err(&'static str),
}
impl Method {
    pub fn from_bytes(inpt: &[u8]) -> Result<Method, MethodError> {
        match inpt.len() {
            3 => match inpt {
                b"GET" => return Ok(Method::GET),
                b"PUT" => return Ok(Method::PUT),
                _ => return Err(MethodError::Err("invalid Method")),
            },
            4 => match inpt {
                b"POST" => return Ok(Method::POST),
                b"HEAD" => return Ok(Method::HEAD),
                _ => return Err(MethodError::Err("invalid Method")),
            },
            5 => match inpt {
                b"TRACE" => return Ok(Method::TRACE),
                b"PATCH" => return Ok(Method::PATCH),
                _ => return Err(MethodError::Err("invalid Method")),
            },
            6 => match inpt {
                b"DELETE" => return Ok(Method::DELETE),
                _ => return Err(MethodError::Err("invalid Method")),
            },
            7 => match inpt {
                b"OPTIONS" => return Ok(Method::OPTIONS),
                b"CONNECT" => return Ok(Method::CONNECT),
                _ => return Err(MethodError::Err("invalid Method")),
            },
            _ => return Err(MethodError::Err("invalid length")),
        }
    }
}
