#!/bin/bash

# Start the backend with debug logging enabled
export RUST_LOG=debug,tower_http=debug
export PORT=8083

echo "Starting backend with debug logging..."
./target/release/backend
