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
*  [ ] Move to a cargo workspace and make this a lib package
*  [ ] Less cloning
*  [ ] Rename Node to Router
*  [ ] Remove Generics constrains on strucs and move them all to impl blocks

### Notes
* This is not made for production applications
