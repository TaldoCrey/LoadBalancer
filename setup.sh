#!/bin/bash

echo "Running, setup!"

if [ -d /sys/class/net/tun0 ]; then
    sudo ip link delete tun0
    echo "Deleted previous tun0..."
fi

echo "Setting up new tun0..."
sudo ip tuntap add dev tun0 mode tun
echo "Addressing ip 10.0.0.1/24..."
sudo ip addr add 10.0.0.1/24 dev tun0
echo "Starting interface..."
sudo ip link set dev tun0 up

echo "tun0 is set!"