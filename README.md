pyenv virtualenv pyrjit-venv
pyenv local pyrjit-venv
pyenv activate pyrjit-venv
pip install maturin
maturin init
# Select pyo3
