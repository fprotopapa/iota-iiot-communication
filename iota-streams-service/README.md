# iota-streams-service

Rust Service to control Iota Streams communication. 
Communication with GRPC. This repository has Server and Client implementation.

## Build

```
cargo build or cross build --target <target>
cargo doc --open
```

Tested:
* cargo v1.56.0
* rustc v1.56.1
* stable
* cross 0.2.1

Architecture:
* x86_64-unknown-linux-gnu
* aarch64-unknown-linux-gnu
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

