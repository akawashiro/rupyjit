#!/bin/bash -eux

if [[ ! -d venv ]]; then
    python3 -m venv venv
fi
source venv/bin/activate
pip install maturin

cargo fmt
maturin develop
export RUST_LOG=info
export PYTHONPATH=$(realpath ./venv/lib/python3.10/site-packages/rupyjit/)
python3 ./test.py
# gdb --ex run --args python3 ./test.py
