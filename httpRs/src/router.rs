#![forbid(unsafe_code)]
use crate::parse;
use crate::parse::NewRequestType;
use crate::request::parse_request;
use crate::request::ToRequest;
use crate::{request::Request, response::IntoResp};
use async_std::sync::Arc;
use bytes::Bytes;
use http::StatusCode;
use std::pin::Pin;
use std::vec;
use std::{collections::HashMap, future::Future};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;
// This list isn't exhaustive will add to it in the future
const LIST_UNSUPPORTED: &[char] = &['.', '&', '(', ')', '=', '}', '{', '$'];

// Middleware definitions
/*
pub type MiddlewareResponse<'a> =
    Pin<Box<dyn Future<Output = Result<(Request, Box<dyn Clone>), StatusCode>> + Send + 'a>>;

pub type MiddleWareFunctionType<T> =
    fn(Request, T, Option<Box<dyn Clone>>) -> MiddlewareResponse<'static>;

*/

pub type HandlerResponse<'a> = Pin<Box<dyn Future<Output = Box<dyn IntoResp + Send>> + Send + 'a>>;
pub type HandlerType = fn(NewRequestType) -> HandlerResponse<'static>;

pub type HandlerTypeState<T> = fn(NewRequestType, T) -> HandlerResponse<'static>;
pub type HandlerTypeStateAndExtract<T> =
    fn(NewRequestType, T, HashMap<String, String>) -> HandlerResponse<'static>;

#[derive(Debug, Default, Clone)]
pub enum Handler<T: std::clone::Clone> {
    #[default]
    None,
    Without(HandlerType),
    WithState(HandlerTypeState<T>),
    WithStateAndExtract(HandlerTypeStateAndExtract<T>),
}
impl<T: std::clone::Clone> Handler<T>
where
    T: Clone,
{
    pub async fn handle(
        self,
        req: NewRequestType,
        state: Option<T>,
        extracts: Option<HashMap<String, String>>,
    ) -> Option<Box<dyn IntoResp + Send>> {
        match self {
            Handler::Without(func) => Some(func(req).await),
            Handler::WithState(func) => match state {
                Some(state) => Some(func(req, state).await),
                None => Some(Box::new((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Missing state".to_string(),
                )) as Box<dyn IntoResp + Send>),
            },
            Self::WithStateAndExtract(func) => {
                let extract = match extracts {
                    None => {
                        return Some(Box::new((
                            StatusCode::BAD_REQUEST,
                            "Missing path extracts".to_string(),
                        )) as Box<dyn IntoResp + Send>)
                    }
                    Some(ext) => ext,
                };
                let state = match state {
                    Some(state) => state,
                    None => {
                        return Some(Box::new((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Missing state".to_string(),
                        )) as Box<dyn IntoResp + Send>)
                    }
                };
                Some(func(req, state, extract).await)
            }

            Self::None => None,
        }
    }
}
pub trait ToHeader {
    fn to_header(&self) -> String;
}
#[derive(Debug)]
pub struct Redirect(pub &'static str);
impl Redirect {
    pub fn new(loc: &'static str) -> Self {
        Self(loc)
    }
}
impl ToHeader for Redirect {
    fn to_header(&self) -> String {
        // you could do input validation that checks wether its a  correct path but this is
        // something for another time
        "Location:".to_owned() + self.0
    }
}
#[derive(Debug, Clone)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}
pub struct Html(pub String);
pub struct Json<T: serde::Serialize>(pub T);
#[derive(Debug)]
//Still need to correctly implement expires and max-age
pub struct Cookie {
    pub name: &'static str,
    pub value: &'static str,
    pub domain: Option<&'static str>,
    pub same_site: Option<SameSite>,
    pub expires: Option<std::time::Duration>,
    pub max_age: Option<u32>,
    pub secure: bool,
    pub http_only: bool,
    pub path: Option<&'static str>,
}
impl ToHeader for Cookie {
    fn to_header(&self) -> String {
        format!(
            "Set-Cookie: {}={};{}{}{}{}{}{}{}",
            self.name,
            self.value,
            match &self.domain {
                Some(domain) => "Domain=".to_owned() + domain + ";",
                None => "".to_string(),
            },
            match &self.same_site {
                Some(same_site) => {
                    match same_site {
                        SameSite::Strict => "SameSite: Strict;",
                        SameSite::Lax => "SameSite: Lax;",
                        SameSite::None => "SameSite: None;",
                    }
                }
                None => "",
            },
            match &self.expires {
                None => "".to_string(),
                Some(expires) => expires.as_nanos().to_string(),
            },
            match self.max_age {
                Some(age) => age.to_string(),
                None => "".to_string(),
            },
            match &self.secure {
                true => "Secure;",
                false => "",
            },
            match &self.http_only {
                true => "HttpOnly;",
                false => "",
            },
            match &self.path {
                Some(path) => "Path=".to_owned() + path + ";",
                None => "".to_string(),
            }
        )
    }
}
impl Cookie {
    pub fn new(name: &'static str, value: &'static str) -> Self {
        Cookie {
            name,
            value,
            domain: None,
            same_site: None,
            expires: None,
            max_age: None,
            secure: true,
            http_only: true,
            path: None,
        }
    }
}

pub struct RoutingResult<T: std::clone::Clone> {
    pub handler: Handler<T>,
    pub extract: Option<HashMap<String, String>>,
}
#[derive(Debug, Default)]
pub struct Router<T: Clone + Default + Send + std::marker::Sync> {
    pub routes: Node<T>,
    pub fallback: Option<Handler<T>>,
}
impl<T> Router<T>
where
    T: Sync,
    T: Clone,
    T: Default,
    T: Send,
    T: std::fmt::Debug,
{
    pub fn new() -> Self {
        Router {
            routes: Node::new("/"),
            fallback: None,
        }
    }
    async fn serve(&'static self, addr: &str) -> ! {
        let listener = match TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(e) => panic!("Cannot create listener Error: {e} "),
        };
        loop {
            let (socket, _) = match listener.accept().await {
                Ok((socket, other_thing)) => (socket, other_thing),
                Err(e) => panic!("Canot accept connection Error: {e}"),
            };

            tokio::spawn(async move {
                //                                                Might want to avoid cloning the
                //                                                state for every connection, maybe
                //                                                an Arc::clone would be better since it
                //                                                does not create new memory
                //
                match handle_conn_node_based(
                    socket,
                    &self.routes,
                    self.fallback.clone(),
                    self.routes.state.clone(),
                )
                .await
                {
                    Ok(_) => (),
                    Err(e) => {
                        panic!("Cannot handle incomming connection: {e} \n")
                    }
                }
            });
        }
    }
    async fn serve_tls(&'static self, addr: &str, path_to_cert: &str) -> ! {
        // need to implement loading the certs
        // This implementation is very close to the example in the tokio_rustls crate
        let cert_chain = crate::tls::load_certificates_from_pem(path_to_cert).unwrap();
        let key_der = crate::tls::load_private_key_from_file(path_to_cert).unwrap();
        let config = match rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key_der)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))
        {
            Err(e) => panic!("{e}"),
            Ok(config) => config,
        };
        let listener = match TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(e) => panic!("Cannot create listener Error: {e} "),
        };
        let acceptor = TlsAcceptor::from(Arc::new(config));
        loop {
            let (socket, _) = match listener.accept().await {
                Ok((socket, other_thing)) => (socket, other_thing),
                Err(e) => panic!("Canot accept connection Error: {e}"),
            };
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                let stream = match acceptor.accept(socket).await {
                    Ok(stream) => stream,
                    Err(e) => panic!("Cannot accept connection {e}"),
                };
                match crate::tls::handle_conn_node_based_tls(
                    tokio_rustls::TlsStream::Server(stream),
                    &self.routes,
                    None,
                    self.routes.state.clone(),
                )
                .await
                {
                    Ok(()) => (),
                    Err(e) => panic!("Cannot handle incomming connection: {e}"),
                };
            });
        }
    }
    pub fn add_handler(
        &mut self,
        path: &str,
        handler: Handler<T>,
    ) -> std::result::Result<Self, ()> {
        let mut router = match self.routes.add_handler(path, handler) {
            Err(_) => return Err(()),
            Ok(r) => r,
        };
        Ok(Router {
            routes: std::mem::take(&mut router),
            fallback: self.fallback.clone(),
        })
    }
    pub fn fallback(mut self, func: Handler<T>) -> Self {
        self.fallback = Some(func);
        self
    }
    pub fn with_state(mut self, state: T) -> Self {
        self.routes.state = Some(state);
        self
    }

    pub fn make_into_serveable(self) -> RouterServable<T> {
        let boxed = Box::new(self);
        let leaked = Box::leak(boxed);
        RouterServable { router: leaked }
    }
}
#[derive(Debug)]
pub struct RouterServable<T: Clone + Default + Send + std::marker::Sync + 'static> {
    router: &'static Router<T>,
}
impl<T> RouterServable<T>
where
    T: Sync,
    T: Clone,
    T: Default,
    T: Send,
    T: std::fmt::Debug,
{
    pub async fn serve(self, addr: &str) -> ! {
        self.router.serve(addr).await
    }
    pub async fn serve_tls(self, addr: &str, path_to_cert: &str) -> ! {
        self.router.serve_tls(addr, path_to_cert).await
    }
}

#[derive(Debug, Default)]
pub struct Node<T: Clone + Default + Send + std::marker::Sync> {
    pub subpath: String,
    pub children: Option<Box<Vec<Box<Node<T>>>>>,
    pub handler: Option<Handler<T>>,
    pub state: Option<T>,
}
impl<T> Node<T>
where
    T: Sync,
    T: Clone,
    T: Default,
    T: Send,
    T: std::fmt::Debug,
{
    pub fn new(path: &str) -> Self {
        Node {
            subpath: path.to_string(),
            children: None,
            handler: None,
            state: None,
        }
    }
    pub fn add_state(&mut self, state: T) -> Self {
        self.state = Some(state);
        return std::mem::take(self);
    }
    pub fn make_into_serveable(self) -> &'static mut Self {
        let box_self = Box::new(self);
        Box::leak(box_self)
    }
    pub async fn serve(&'static self, addr: &str) -> ! {
        let listener = match TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(e) => panic!("Cannot create listener Error: {e} "),
        };
        loop {
            let (socket, _) = match listener.accept().await {
                Ok((socket, other_thing)) => (socket, other_thing),
                Err(e) => panic!("Canot accept connection Error: {e}"),
            };

            tokio::spawn(async move {
                //                                                Might want to avoid cloning the
                //                                                state for every connection, maybe
                //                                                an Arc::clone would be better since it
                //                                                does not create new memory
                //
                match handle_conn_node_based(socket, &self, None, self.state.clone()).await {
                    Ok(_) => (),
                    Err(e) => {
                        panic!("Cannot handle incomming connection: {e} \n")
                    }
                }
            });
        }
    }
    pub fn get_handler(&self, path: String) -> Option<RoutingResult<T>> {
        if path == "/" {
            match &self.handler {
                Some(handler) => {
                    return Some(RoutingResult {
                        // Would like to avoid cloing this for every connection
                        handler: handler.clone(),
                        extract: None,
                    });
                }
                None => return None,
            }
        }
        match pub_walk(&self.children, path) {
            Some(handler) => return Some(handler),
            None => return None,
        }
    }
    pub fn add_handler(
        &mut self,
        path: &str,
        handler: Handler<T>,
    ) -> std::result::Result<Box<Self>, ()> {
        if path == "/" {
            self.handler = Some(handler);
            return Ok(Box::new(std::mem::take(self)));
        }
        for i in LIST_UNSUPPORTED.into_iter() {
            if path.contains(*i) {
                panic!(
                    "Your path contains a value that is unsupported. Char: {}",
                    i
                );
            }
        }

        let res = pub_walk_add_node(self, path.to_string(), handler);
        if let Some((node, ok)) = res {
            if ok {
                match self.children.as_mut() {
                    None => self.children = Some(Box::new(vec![node])),
                    Some(vec) => {
                        // this is ugly
                        vec.push(node);
                        // this is due to the std::mem::take that leaves a default in there
                        if let Some(first_node) = vec.get(0) {
                            if first_node.subpath == "" {
                                vec.remove(0);
                            }
                        }
                        return Ok(Box::new(std::mem::take(self)));
                    }
                }
                return Ok(Box::new(std::mem::take(self)));
            }
            return Ok(node);
        } else {
            return Ok(Box::new(std::mem::take(self)));
        }
    }
    pub fn insert(&mut self, path: String, path_rn: String, func: Handler<T>) -> Box<Node<T>> {
        //This is the base case when the path is reached the node is returned
        if path == path_rn {
            self.handler = Some(func);
            return Box::new(std::mem::take(self));
        }

        let path_for_new_node = match path_rn.as_str() {
            "/" => {
                let splits: Vec<String> = path.split("/").map(|split| split.to_string()).collect();
                let mut to_add = String::new();
                for i in splits.into_iter() {
                    if i == "" {
                        continue;
                    }
                    to_add = i.clone();
                    break;
                }
                let final_str = "/".to_string() + to_add.as_str();
                final_str
            }
            _ => {
                let missing_part_of_path = path.replace(path_rn.clone().as_str(), "").to_string();

                let splits: Vec<String> = missing_part_of_path
                    .split("/")
                    .map(|split| split.to_string())
                    .collect();
                let mut to_add_to_curr = String::new();
                for i in splits.into_iter() {
                    if i == "" {
                        continue;
                    }
                    to_add_to_curr = i.clone();
                    break;
                }
                let mut final_path = match path_rn.ends_with("/") {
                    false => path_rn.clone() + "/" + to_add_to_curr.as_str(),
                    true => path_rn.clone() + to_add_to_curr.as_str(),
                };
                if to_add_to_curr == "" {
                    final_path = missing_part_of_path;
                }
                final_path
            }
        };
        let mut new_node = Node::new(path_for_new_node.as_str());
        match self.children.as_mut() {
            Some(children) => {
                let node = new_node.insert(path.clone(), path_for_new_node.clone(), func);
                children.push(node);
                return Box::new(std::mem::take(self));
            }
            None => {
                //Recursivly insert until the path is reached
                let node = new_node.insert(path.clone(), path_for_new_node.clone(), func);
                let mut new_vec = Vec::new();
                new_vec.push(node);
                let boxed_vec = Box::new(new_vec);
                self.children = Some(boxed_vec);
                return Box::new(std::mem::take(self));
            }
        };
    }
}

fn pub_walk_add_node<
    T: std::default::Default
        + std::clone::Clone
        + std::marker::Send
        + std::marker::Sync
        + std::fmt::Debug,
>(
    node: &mut Node<T>,
    path: String,
    func: Handler<T>,
) -> Option<(Box<Node<T>>, bool)> {
    match node.children.as_mut() {
        Some(children) => {
            let mut matches = 0;
            for i in 0..children.len() {
                let child = children.get_mut(i)?;
                if child.subpath == path {
                    child.handler = Some(func);
                    return Some((Box::new(std::mem::take(node)), false));
                }
                let test_str = child.subpath.clone() + "/";
                if path.starts_with(test_str.as_str()) {
                    // All of the below might have been fixed
                    // This causes a bug since /wow also matches on /wowo
                    // Another situation that becomes an issue is when we have path /cool/wow
                    // already added and want to add /user/:id/cool/ts/:ts
                    // this happens since /cool also appears in /user/:id/cool/ts/:ts
                    // this is not wanted since you obv should not append to /wowo
                    matches = matches + 1;

                    match pub_walk_add_node(child, path.clone(), func) {
                        Some((node, ok)) => return Some((node, ok)),
                        None => return None,
                    };
                }
            }
            if matches == 0 {
                let node_path_curr = node.subpath.clone();
                let node = node.insert(path, node_path_curr, func);

                return Some((node, true));
            }
            return None;
        }
        None => {
            let node_path_curr = node.subpath.clone();
            //let node = node.insert(path, node_path_curr, func);
            let node = node.insert(path, node_path_curr, func);
            return Some((node, false));
        }
    }
}

fn pub_walk<
    T: std::default::Default + std::clone::Clone + std::marker::Send + std::marker::Sync,
>(
    children: &Option<Box<Vec<Box<Node<T>>>>>,
    path: String,
) -> Option<RoutingResult<T>> {
    if let Some(children) = children {
        for child in children.as_ref().into_iter() {
            if child.subpath.contains(":") {
                // This generic path extract handling isn't very well optimized
                // Need to work on this
                let splits: Vec<String> = child
                    .subpath
                    .split(":")
                    .map(|split| split.to_string())
                    .collect();
                if splits.len() != 2 {
                    //This path has more than one generic extract
                    // for example /user/:id/time/:ts

                    let child_path_splits: Vec<String> = child
                        .subpath
                        .split("/")
                        .map(|split| split.to_string())
                        .collect();
                    let curr_path_split: Vec<String> =
                        path.split("/").map(|split| split.to_string()).collect();
                    if curr_path_split.len() != child_path_splits.len() {
                        // if the path is longer than the one rn we continue the search
                        match pub_walk(&child.children, path.clone()) {
                            Some(handler) => return Some(handler),
                            None => (),
                        }
                        continue;
                    }
                    // switched the extracts to be an HashMap since it is now more than one
                    // extract
                    let mut extracts: HashMap<String, String> = HashMap::new();
                    'inner: for (i, val) in child_path_splits.into_iter().enumerate() {
                        if val.starts_with(":") {
                            let extract = curr_path_split.get(i)?;
                            extracts.insert(val.replace(":", "").to_string(), extract.to_string());
                            continue 'inner;
                        }
                    }
                    let handler = match &child.handler {
                        Some(handler) => handler,
                        None => {
                            continue;
                        }
                    };
                    return Some(RoutingResult {
                        // Would like to avoid cloing this for every connection
                        handler: handler.clone(),
                        extract: Some(extracts),
                    });
                }
                // This is the identifier to the extract. For instance if we registered the route
                // /user/:id then split by ":"  at index 0 we get the path before the ":" and at index 1 the identifier
                // or how the user should be abled to extract it in his handler
                let identifier = splits.get(1)?;
                let path_before = splits.get(0)?;
                if path.contains(path_before) {
                    let values_split: Vec<String> = path
                        .split(path_before)
                        .map(|split| split.to_string())
                        .collect();
                    if values_split.len() != 2 {
                        continue;
                    }
                    let value = values_split.get(1)?;
                    // This will likely become a HashMap since we
                    // want to have to ability to handle multiple extractors

                    let mut extracts = HashMap::new();
                    extracts.insert(identifier.clone(), value.clone());
                    match &child.handler {
                        Some(handler) => {
                            return Some(RoutingResult {
                                handler: handler.clone(),
                                extract: Some(extracts),
                            });
                        }
                        None => {
                            match pub_walk(&child.children, path.clone()) {
                                Some(handler) => return Some(handler),
                                None => (),
                            };
                            ()
                        }
                    }
                }
            }
            if child.subpath == path {
                match &child.handler {
                    Some(handler) => {
                        return Some(RoutingResult {
                            // Would like to avoid cloing this for every connection
                            handler: handler.clone(),
                            extract: None,
                        });
                    }
                    None => (),
                }
            }
            if path.contains(child.subpath.as_str()) {
                let new_children = &child.children;
                match pub_walk(new_children, path.clone()) {
                    Some(handler) => return Some(handler),
                    None => (),
                };
            }
        }
    }
    None
}
pub async fn send_error_response(mut socket: TcpStream, code: StatusCode) -> std::io::Result<()> {
    let res = code.into_response();
    socket.write_all(res.as_slice()).await?;
    socket.flush().await?;
    socket.shutdown().await?;
    Ok(())
}
pub async fn handle_conn_node_based<
    T: std::clone::Clone
        + std::default::Default
        + std::marker::Send
        + std::marker::Sync
        + std::fmt::Debug,
>(
    mut socket: TcpStream,
    handlers: &Node<T>,
    fallback: Option<Handler<T>>,
    state: Option<T>,
) -> std::io::Result<()> {
    // did this to avoid leftover zeros at the end;
    let mut buf = Vec::with_capacity(1024);
    socket.read_buf(&mut buf).await?;
    let req_str = String::from_utf8_lossy(&buf[..]);
    let res = parse::parse_request(&req_str).unwrap();

    let routing_res: RoutingResult<T> = match handlers.get_handler(res.metadata.path.clone()) {
        Some(res) => res,
        None => match fallback {
            Some(fallback) => {
                let res = match fallback.handle(res, state, None).await {
                    Some(res) => res,
                    None => {
                        send_error_response(socket, StatusCode::NOT_FOUND).await?;
                        return Ok(());
                    }
                };
                let resp = res.into_response();
                socket.write_all(resp.as_slice()).await?;
                socket.flush().await?;
                socket.shutdown().await?;
                return Ok(());
            }
            None => {
                send_error_response(socket, StatusCode::NOT_FOUND).await?;
                return Ok(());
            }
        },
    };
    let handler = routing_res.handler;

    // will try to find another solution other to cloning this map
    let map_clone = res.params.clone();
    let res = match handler
        .handle(
            res,
            state,
            // This is needed since there are two ways extracts can be added to the request
            // The first being for example /user/:id which comes from the router
            // And the second being on the end of the request path for example
            // /user/:id?page=10
            // This gets parsed by the request parser so we need to merge the two maps
            match routing_res.extract {
                Some(mut res) => match map_clone {
                    Some(map2) => {
                        res.extend(map2.into_iter());
                        Some(res)
                    }

                    None => Some(res),
                },
                None => None,
            },
        )
        .await
    {
        Some(res) => res,
        None => return send_error_response(socket, StatusCode::NOT_FOUND).await,
    };
    let response = res.into_response();

    socket.write_all(response.as_slice()).await?;

    socket.flush().await?;

    socket.shutdown().await?;
    Ok(())
}
