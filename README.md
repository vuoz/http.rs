# http.rs


Just another web framework but in rust

## Goals
* Understand the Http Protocol  
* Getting a deeper understanding of Rust  

## Things on the agenda  
* Implement regex based routing  
    * This requires the addition of extractors to make use of the parameters in the Uri  
* Simplify the api  
* Fix bugs in the rucursive addition and traversal of Nodes  
* Even tho already metioned I want to highlight that I would  
like to simplify the creation of handlers  
* Make a state extractor so that handler can use state  
* Find a better name  
* Might want to implement a thread pool instead of spawning a new thread  
for every request  
* Middleware  

### Notes
* This is not made for production applications
