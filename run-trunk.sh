#!/bin/bash

PORT=8001

# Check if anything is using the port
if lsof -Pi :$PORT -sTCP:LISTEN -t &>/dev/null; then
    PID=$(lsof -Pi :$PORT -sTCP:LISTEN -t)
    PROCESS_NAME=$(ps -p $PID -o comm= 2>/dev/null)
    
    # Check if it's a trunk process
    if [[ "$PROCESS_NAME" == "trunk" ]] || ps -p $PID -o cmd= 2>/dev/null | grep -q "trunk"; then
        echo "Found trunk process on port $PORT (PID: $PID). Killing it..."
        kill $PID
        sleep 1
        # Forcefully kill if still running
        if kill -0 $PID 2>/dev/null; then
            kill -9 $PID
        fi
        echo "Killed trunk process"
    else
        echo "Error: Port $PORT is already in use by '$PROCESS_NAME' (PID: $PID)"
        echo "This is not a trunk process, so exiting to avoid conflicts."
        exit 1
    fi
fi

echo "Starting trunk on port $PORT in background..."
trunk serve \
  --port $PORT \
  --address 0.0.0.0 \
  --release &

TRUNK_PID=$!
echo "Trunk started with PID: $TRUNK_PID"
echo "To stop: kill $TRUNK_PID"
