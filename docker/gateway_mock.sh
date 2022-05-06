#!/bin/bash

./mqtt-server &
./iota-identity-server &
./iota-streams-server &
( sleep 120 ; ./mock-client ) &
RUST_LOG=info ./gateway &
