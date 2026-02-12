#!/usr/bin/env bash

# FILE="./tetris.gb"
FILE="./cpu_instrs.gb"
# FILE="./pokemon.gb"
# FILE="./pokemon_silver.gbc"

cargo build

cargo run -- "$FILE"
