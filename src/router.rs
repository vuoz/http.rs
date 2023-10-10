use crate::parse_request;
use crate::{response::IntoResp, Request};
use http::StatusCode;
use std::pin::Pin;
use std::{collections::HashMap, future::Future};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

// Definition of the handler types
pub type HandlerResponse<'a> = Pin<Box<dyn Future<Output = Box<dyn IntoResp + Send>> + Send + 'a>>;
pub type MiddlewareResponse<'a> =
    Pin<Box<dyn Future<Output = Result<(Request, Box<dyn Clone>), StatusCode>> + Send + 'a>>;

pub type MiddleWareFunctionType<T> =
    fn(Request, T, Option<Box<dyn Clone>>) -> MiddlewareResponse<'static>;

pub type HandlerType = fn(Request) -> HandlerResponse<'static>;

pub type HandlerTypeState<T> = fn(Request, T) -> HandlerResponse<'static>;

//pub type HandlerTypeStateAndExtract<T, S> = fn(Request, Json<S>, T) -> HandlerResponse<'static>;

#[derive(Debug, Default, Clone)]
pub enum Handler<T: std::clone::Clone> {
    #[default]
    None,
    Without(HandlerType),
    WithState(HandlerTypeState<T>),
}
impl<T: std::clone::Clone> Handler<T>
where
    T: Clone,
{
    pub async fn handle(self, req: Request, state: Option<T>) -> Option<Box<dyn IntoResp + Send>> {
        match self {
            Handler::Without(func) => Some(func(req).await),
            Handler::WithState(func) => match state {
                Some(state) => Some(func(req, state).await),
                None => Some(Box::new((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Missing state".to_string(),
                )) as Box<dyn IntoResp + Send>),
            },

            Self::None => None,
        }
    }
}

pub struct RoutingResult<T: std::clone::Clone> {
    pub handler: Handler<T>,
    pub extract: Option<HashMap<String, String>>,
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
    pub fn new(path: String) -> Self {
        Node {
            subpath: path,
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
    pub async fn serve(&'static self, addr: String) -> ! {
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
                match handle_conn_node_based(socket, &self, None, self.state.clone()).await {
                    Ok(_) => (),
                    Err(e) => {
                        panic!("Cannot handle incomming connection: {e}")
                    }
                };
            });
        }
    }
    pub fn get_handler(&self, path: String) -> Option<RoutingResult<T>> {
        if path == "/" {
            match &self.handler {
                Some(handler) => {
                    return Some(RoutingResult {
                        handler: handler.clone(),
                        extract: None,
                    })
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
        path: String,
        handler: Handler<T>,
    ) -> std::result::Result<Box<Self>, ()> {
        if path == "/" {
            self.handler = Some(handler);
            return Ok(Box::new(std::mem::take(self)));
        }

        let res = pub_walk_add_node(self, path, handler);
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
        let mut new_node = Node::new(path_for_new_node.clone());
        match self.children.as_mut() {
            Some(children) => {
                let node = new_node.insert(path.clone(), path_for_new_node.clone(), func);
                children.push(node);
                return Box::new(std::mem::take(self));
            }
            None => {
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
                            handler: handler.clone(),
                            extract: None,
                        })
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
    socket.write(res.as_slice()).await?;
    socket.flush().await?;
    return Ok(());
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
    fallback: Option<HandlerType>,
    state: Option<T>,
) -> std::io::Result<()> {
    let mut buf = [0; 1024];
    socket.read(&mut buf).await?;
    let req_str = String::from_utf8_lossy(&buf[..]);
    let mut request = match parse_request(req_str) {
        Ok(request) => request,
        Err(_) => {
            send_error_response(socket, StatusCode::BAD_REQUEST).await?;
            return Ok(());
        }
    };

    let routing_res = match handlers.get_handler(request.metadata.path.clone()) {
        Some(res) => res,
        None => match fallback {
            Some(fallback) => {
                let res = fallback(request).await;
                let resp = res.into_response();
                socket.write(resp.as_slice()).await?;
                socket.flush().await?;
                return Ok(());
            }
            None => {
                let res = StatusCode::NOT_FOUND.into_response();
                socket.write(res.as_slice()).await?;
                socket.flush().await?;
                return Ok(());
            }
        },
    };
    let handler = routing_res.handler;
    if let Some(extract) = routing_res.extract {
        request.extract = Some(extract);
    }
    let res = match handler.handle(request, state).await {
        Some(res) => res,
        None => {
            let res = StatusCode::NOT_FOUND.into_response();
            socket.write(res.as_slice()).await?;
            socket.flush().await?;
            return Ok(());
        }
    };
    let response = res.into_response();
    let clone = response.clone();
    socket.write(clone.as_slice()).await?;
    socket.flush().await?;

    return Ok(());
}
