# rust-iota-secure-communication

## Docker

IoT Gateway + Mock Sensor

```
docker build -t gateway_mock -f docker/Dockerfile.gatewaymock .

docker run --env-file docker/.env_gatewaymock -v $(pwd)/docker/storage/gatewaymock:/gateway_mock/storage -it gateway_mock /bin/bash

```

Client Application (Factory)

```
docker build -t client_factory -f docker/Dockerfile.clientfactory .

docker run --env-file docker/.env_clientfactory -v $(pwd)/docker/storage/clientfactory:/client_factory/storage -it client_factory /bin/bash
```

Client Application (Vendor)

```
docker build -t client_vendor -f docker/Dockerfile.clientvendor .

docker run --env-file docker/.env_clientvendor -v $(pwd)/docker/storage/clientvendor:/client_vendor/storage -it client_vendor /bin/bash
```

IoT Gateway + OPC-UA Server

```
docker build -t gateway_opcua -f docker/Dockerfile.gatewayopcua .

docker run --env-file docker/.env_gatewayopcua -v $(pwd)/docker/storage/gatewayopcua:/gateway_opcua/storage -it gateway_opcua /bin/bash
```

IoT Gateway + Document

```
docker build -t gateway_doc -f docker/Dockerfile.gatewaydoc .

docker run --env-file docker/.env_gatewaydoc -v $(pwd)/docker/storage/gatewaydoc:/gateway_doc/storage -it gateway_doc /bin/bash
```