# rust-mqtt-service

Rust Service to interact with MQTT Broker. 
Communication over GRPC and MQTT. This repository has Server and Client implementation.

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
* armv7-unknown-linux-gnueabihf
* aarch64-unknown-linux-gnu

## Run

Start Server

```
./target/debug/mqtt-server
```

Start Client

```
./target/debug/mqtt-client
```

