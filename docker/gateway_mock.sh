#!/bin/bash

./mqtt-server &
./iota-identity-server &
./iota-streams-server &

FILE=./storage/database.db
test -e "$FILE" && echo 'Restart ...' && ( sleep 10 ; ./mock-client ) &
! test -e "$FILE" && echo 'First Startup ...' && ( sleep 120 ; ./mock-client ) &

RUST_LOG=info ./gateway &
