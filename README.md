# http.rs


Just another web framework but in rust

## Goals
* Understand the Http Protocol  
* Getting a deeper understanding of Rust  

## Core Features

Handlers with path extracts  

```rust
let Router = Node::new("/")
    .add_handler("/user/:id/ts/:time", router::Handler::Without(test_handler))
    .unwrap();
```

Handlers with State  

```rust
let Router = Node::new("/")
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

        let resp_obj = JsonTest {
            test_string: data.test_string,
            page: data.page,
        };
        respond(Json(resp_obj))
    })
}
```

## Things on the agenda  
* [ ] Implement regex based routing  
    *  [x] This requires the addition of extractors to make use of the parameters in the Uri  
* [ ] Simplify the api  
* [ ] Fix bugs in the rucursive addition and traversal of Nodes   
* [x] Make a state extractor so that handlers can use state  
*  [ ] Find a better name  
*  [ ] Might want to implement a thread pool instead of spawning a new thread  
for every request  
* [ ] Middleware  
* [x]  Mutlitple extracts in one path for example: "/user/:id/time/:ts"
*  [x] Move to a cargo workspace and make this a lib package
*  [ ] Less cloning
*  [ ] Rename Node to Router
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

### Notes
* This is not made for production applications
