#!/usr/bin/env bash

FILE="./pokemon.gb"
# FILE="./pokemon_silver.gbc"

cargo build

cargo run -- "$FILE"
