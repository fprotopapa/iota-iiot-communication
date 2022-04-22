#!/bin/bash
# sudo lsof -i -P -n | grep LISTEN

fuser -k 50054/tcp
fuser -k 50053/tcp
fuser -k 50052/tcp
fuser -k 50051/tcp