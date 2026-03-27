#!/bin/bash

cd balancer


echo "Starting balancer!"

cargo run&

echo "Balancer started!"

sleep 2

cd ../back

echo "Starting servers"

IP="127.0.0"

for i in {100..105}; do
	echo "Starting Server@$IP.$i"
	cargo run -- $IP.$i&
	sleep 2
done

trap "../clear.sh" SIGINT

wait



