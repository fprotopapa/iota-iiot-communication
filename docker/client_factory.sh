#!/bin/bash


cd ./rust-mqtt-service && ./target/debug/mqtt-server &

cd ./iota-identity-service && ./target/debug/iota-identity-server &

cd ./iota-streams-service && ./target/debug/iota-streams-server &

cd ./client && RUST_LOG=info ./target/debug/gateway


# ./mqtt-server &
# ./iota-identity-server &
# ./iota-streams-server &
# ( sleep 10 ; ./mock-client ) &
# RUST_LOG=info ./gateway
