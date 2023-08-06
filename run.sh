#!/bin/bash -eux

# Run this before run this script:
# pyenv activate
# pip install maturin

maturin develop
RUST_LOG=info python3 ./use_rupyjit.py
