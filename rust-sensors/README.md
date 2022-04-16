# rust-sensors

Sensor implementations to use with GRPC. 

* Mock Sensor publishing temperature and humidity
* Bluetooth (or other wireless communication) sending temperature data over tty port
* OPC-UA Server sending data from four temperature sensors

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

## Use Examples

```
git clone https://github.com/fprotopapa/rust-sensors.git && cd rust-sensors
cargo build
```

Run Bluetooth Example:

[README](/bluetooth/README.md)

Run Mock Example:

[README](/mock/README.md)

Run OPC-UA Example:

[README](/opc-ua/README.md)


