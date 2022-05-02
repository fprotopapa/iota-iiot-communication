# rust-iota-secure-communication

## Docker

IoT Gateway + Mock Sensor

```
docker build -t gateway_mock -f docker/Dockerfile.gatewaymock .

docker run --env-file docker/.env_gatewaymock -v $(pwd)/docker/db/gatewaymock:/gateway_mock/db -it gateway_mock /bin/bash

```

Client Application (Factory)

```
docker build -t client_factory -f docker/Dockerfile.clientfactory .

docker run --env-file docker/.env_clientfactory -v $(pwd)/docker/db/clientfactory:/client_factory/db -it client_factory /bin/bash
```

Client Application (Vendor)

```
docker build -t client_vendor -f docker/Dockerfile.clientvendor .

docker run --env-file docker/.env_clientvendor -v $(pwd)/docker/db/clientvendor:/client_vendor/db -it client_vendor /bin/bash
```

IoT Gateway + OPC-UA Server

```
docker build -t gateway_opcua -f docker/Dockerfile.gatewayopcua .

docker run --env-file docker/.env_gatewayopcua -v $(pwd)/docker/db/gatewayopcua:/gateway_opcua/db -it gateway_opcua /bin/bash
```

IoT Gateway + Document

```
docker build -t gateway_doc -f docker/Dockerfile.gatewaydoc .

docker run --env-file docker/.env_gatewaydoc -v $(pwd)/docker/db/gatewaydoc:/gateway_doc/db -it gateway_doc /bin/bash
```