#!/bin/bash -eux

if [[ ! -d "venv-pyjion" ]]; then
    python3 -m venv venv-pyjion
fi
source venv-pyjion/bin/activate
pip install pyjion distorm3 rich

DOTNET_ROOT=/usr/lib/dotnet python3 ./check_pyjion.py
