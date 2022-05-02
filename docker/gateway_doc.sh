#!/bin/bash

./mqtt-server &
./iota-identity-server &
./iota-streams-server &
( sleep 180 ; ./doc-client ) &
./gateway &
