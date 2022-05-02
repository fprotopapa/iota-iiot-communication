#!/bin/bash

./mqtt-server &
./iota-identity-server &
./iota-streams-server &
./opcua-server &
( sleep 50 ; ./opcua-client ) &
./gateway &
