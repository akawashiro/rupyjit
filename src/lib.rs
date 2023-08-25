use chrono;
use log::info;
use std::io::Write;

extern crate num;
#[macro_use]
extern crate num_derive;

use pyo3::ffi::{
    PyFrameObject, PyInterpreterState_Get, PyObject, PyThreadState,
    _PyInterpreterState_GetEvalFrameFunc, _PyInterpreterState_SetEvalFrameFunc,
};
use pyo3::prelude::*;

mod pyutils;
use pyutils::{dump_frame_info, exec_jit_code};

static mut ORIGINAL_FRAME: Option<
    extern "C" fn(state: *mut PyThreadState, frame: *mut PyFrameObject, c: i32) -> *mut PyObject,
> = None;

#[pyfunction]
fn version() -> PyResult<String> {
    Ok(format!(
        "{}.{}.{}",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH")
    ))
}

extern "C" fn eval(state: *mut PyThreadState, frame: *mut PyFrameObject, c: i32) -> *mut PyObject {
    info!("eval()");

    dump_frame_info(state, frame, c);
    exec_jit_code(state, frame, c);

    unsafe {
        if let Some(original) = ORIGINAL_FRAME {
            original(state, frame, c)
        } else {
            panic!("original frame not found");
        }
    }
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
    env_logger::Builder::new()
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
        .filter(None, log::LevelFilter::Info)
        .init();
    m.add_function(wrap_pyfunction!(enable, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
