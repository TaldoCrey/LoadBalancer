#!/bin/bash

echo "Starting tests..."

for i in {2..15}; do

	iphost="127.0.0.$i"
	echo "Sending request from -> $iphost"
	echo "Sending from $iphost" | nc -w 1 -s $iphost 127.0.0.1 2006

	sleep 1

done

echo "Done!"
