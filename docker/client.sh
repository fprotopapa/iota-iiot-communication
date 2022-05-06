#!/bin/bash

./mqtt-server &
./iota-identity-server &
./iota-streams-server &
RUST_LOG=info ./client
