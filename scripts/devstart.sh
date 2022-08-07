#!/usr/bin/env bash

env=${1:-debug}
echo "Building metablockchain node : selected_target : $env"

./target/$env/metablockchain-node --dev --tmp



