#!/bin/bash

cd ./rust-mqtt-service && ./target/debug/mqtt-server &

cd ./iota-identity-service && ./target/debug/iota-identity-server &

cd ./iota-streams-service && ./target/debug/iota-streams-server &
