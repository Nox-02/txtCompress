#!/usr/bin/env sh
set -e
echo "Installing txtCompress..."
if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust is not installed. Get it from https://rustup.rs and run this again."
    exit 1
fi
cargo install --path . --force
echo
echo "Done! Open a new terminal and try:  txtCompress -h"
echo "(If the command is not found, add ~/.cargo/bin to your PATH.)"
