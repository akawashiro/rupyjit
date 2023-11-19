use chrono;
use log::info;
use std::env;
use std::io::Write;

extern crate num;
#[macro_use]
extern crate num_derive;

use pyo3::ffi::{
    PyFrameObject, PyInterpreterState_Get, PyObject, PyThreadState,
    _PyInterpreterState_GetEvalFrameFunc, _PyInterpreterState_SetEvalFrameFunc,
};
use pyo3::prelude::*;

mod jit;
use jit::{eval, ORIGINAL_FRAME};

#[pyfunction]
fn version() -> PyResult<String> {
    Ok(format!(
        "{}.{}.{}",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH")
    ))
}

#[pyfunction]
fn enable() -> PyResult<()> {
    info!("enable()");
    let state = unsafe { PyInterpreterState_Get() };
    unsafe { ORIGINAL_FRAME = Some(_PyInterpreterState_GetEvalFrameFunc(state)) };
    unsafe { _PyInterpreterState_SetEvalFrameFunc(state, eval) };
    Ok(())
}

/// A Python module implemented in Rust.
#[pymodule]
fn rupyjit(_py: Python, m: &PyModule) -> PyResult<()> {
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} {} [{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();
    m.add_function(wrap_pyfunction!(enable, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
