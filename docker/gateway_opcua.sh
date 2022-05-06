#!/bin/bash

./mqtt-server &
./iota-identity-server &
./iota-streams-server &
./opcua-server &

FILE=./storage/database.db
test -e "$FILE" && echo 'Restart ...' && ( sleep 10 ; ./opcua-client ) &
! test -e "$FILE" && echo 'First Startup ...' && ( sleep 120 ; ./opcua-client ) &

RUST_LOG=info ./gateway &
