#!/bin/bash

./mqtt-server &
./iota-identity-server &
./iota-streams-server &
./client
