#!/bin/bash -eux

if [[ ! -d venv ]]; then
    python3 -m venv venv
fi
source venv/bin/activate
pip install maturin

cargo fmt
maturin develop
RUST_LOG=info python3 ./use_rupyjit.py
