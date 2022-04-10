# iota-identity-service

Rust Service to interact with Iota Identity. 
Communication over GRPC. This repository has Server and Client implementation.

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
* armv7-unknown-linux-gnueabihf

## Run

Start Server

```
./target/debug/iota-identity-server
```

Start Client

```
./target/debug/iota-identity-client
```