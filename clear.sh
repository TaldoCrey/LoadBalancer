#!/bin/bash

echo "Clearing processes!"

TARGETS=("back" "balancer")

for NAME in "${TARGETS[@]}"; do

	echo "Searching $NAME"

	if pgrep -x "$NAME" > /dev/null; then
		echo "Encerrando $NAME"
		PIDS=($(pgrep -x $NAME))
		for PID in "${PIDS[@]}"; do
			echo "PID = $PID"
			kill $PID
		done
	else
		echo "Processo não encontrado."
	fi

	sleep 1

done

ps

echo "Completed!"
