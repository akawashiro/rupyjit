#!/bin/bash -eux

if [[ ! -d venv ]]; then
    python3 -m venv venv
fi
source venv/bin/activate
pip install maturin

cargo fmt
maturin develop
export PYTHONPATH=$(realpath ./venv/lib/python3.10/site-packages/rupyjit/)

# export RUST_LOG=info
# TEST_PYTHON_FILES=$(find . -name "test_*.py")
# for TEST_PYTHON_FILE in $TEST_PYTHON_FILES; do
#     python3 $TEST_PYTHON_FILE
#     echo "Passed $TEST_PYTHON_FILE"
# done

export RUST_LOG=info
python3 ./test.py
# gdb --ex run --args python3 ./test.py
