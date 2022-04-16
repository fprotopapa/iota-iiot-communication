# Bluetooth Sensor over Serial

Terminal 1:

```
sudo socat PTY,link=/dev/ttyBleIn PTY,link=/dev/ttyBleOut &
sudo chmod o+rw /dev/ttyBleIn
./target/debug/bluetooth-server
```

Terminal 2:

```
sudo ./target/debug/bluetooth-client
```

Terminal 3:

```
echo "{\"temperature\": 10}" > /dev/ttyBleIn
```