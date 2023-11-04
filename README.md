# http.rs


Just another web framework but in rust

## Goals
* Understand the Http Protocol  
* Getting a deeper understanding of Rust  

## Core Features

Handlers with path extracts  

```rust
let router = Router::new()
    .add_handler("/user/:id/ts/:time", router::Handler::Without(test_handler))
    .unwrap();
```

Handlers with State  

```rust
let router = Router::new()
    .add_handler("/user/:id/ts/:time", router::Handler::WithState(test_handler))
    .unwrap()
    .add_state(AppState {...});
```


Deserializing your request body to a json struct with one function call
 
```rust
fn test_handler(
    req: Request,
) -> HandlerResponse<'static> {
    Box::pin(async move {
        let data: JsonTest = req.from_json_to_struct().unwrap();
        {...}
    })
}
```

Request handler with state access

```rust
#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub hello_page: String,
}

fn test_handler(
    _req: Request,
    state: AppState,//<---------
) -> HandlerResponse<'static> {
    Box::pin(async move {
        respond(Html(state.hello_page))
    })
}
```
Easily respond with JSON

```rust
fn test_handler(
    req:Request
)-> HandlerResponse<'static>{
    Box::pin(async move {
        respond(Json(YourJsonStruct{...}))
    })
}
```
or HTML
```rust
fn test_handler(
    req:Request
)-> HandlerResponse<'static>{
    Box::pin(async move {
        respond(Html(...))
    })
}
```

Easily respond with StatusCodes
```rust
fn test_handler(
    req:Request
)-> HandlerResponse<'static>{
    Box::pin(async move {
        respond(StatusCode::Ok)
    })
}
```

## Things on the agenda  
* [ ] Comply with Rfc standard
* [ ] Implement regex based routing  
    *  [x] This requires the addition of extractors to make use of the parameters in the Uri  
* [ ] Simplify the api  
* [ ] Fix bugs in the recursive addition and traversal of Routers   
* [x] Make a state extractor so that handlers can use state  
*  [ ] Find a better name  
*  [ ] Might want to implement a thread pool instead of spawning a new thread  
for every request  
* [ ] Middleware  
* [x]  Mutlitple extracts in one path for example: ```"/user/:id/time/:ts"```
*  [x] Move to a cargo workspace and make this a lib package
*  [ ] Less cloning
*  [ ] Rename Router to Router
*  [ ] Move all generics constrians to impl blocks
*  [x] Simplify returning Html and Json
    * [x] Html 
    * [x] Json  
* [ ] TLS
* [ ] Other HTTP Versions
   * [ ] HTTP 2
   * [ ] HTTP 3
* [ ] Improve overall code quality
* [ ] Correct Content-length handling 
* [ ] Correct Connnection: close handling
* [ ] Chunked transfer
* [ ] Timeout requests
* [ ] Simplify Set-Cookie
* [ ] Simplify Redirecting
* [ ] Add tests
* [ ] Encoding & Compression
* [x] Cookie access
* [ ] Proc macro that implements IntoResp for all tuple combinations

### Notes
* This is not made for production applications
