### Dependencies
The `wasm-pack` binary can be installed with
```
cargo install wasm-pack
```

A static http server is needed to serve the files. `http` can be installed with
```
cargo install https
```

### Building
The project can be built with
```
wasm-pack build --target web 
```

and served on any static http server
```
http 
```
