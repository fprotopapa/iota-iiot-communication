# iota-streams-service

Rust Service to control Iota Streams communication. 
Communication with GRPC. This repository has Server and Client implementation.

## Build

```
cargo build
cargo doc --open
```

Tested:
* cargo v1.56.0
* rustc v1.56.1
* stable

Architecture:
* x86_64-unknown-linux-gnu
* armv7-unknown-linux-musleabihf
* armv7-unknown-linux-gnueabihf

## Run

Start Server

```
./target/debug/iota-streams-server
```

Start Client

```
./target/debug/iota-streams-client
```

