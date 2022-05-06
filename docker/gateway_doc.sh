#!/bin/bash

./mqtt-server &
./iota-identity-server &
./iota-streams-server &
( sleep 180 ; ./doc-client ) &
RUST_LOG=info ./gateway &
