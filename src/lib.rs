use log::info;
use pyo3::ffi::{
    PyBytes_AsString, PyBytes_Check, PyBytes_Size, PyFrameObject, PyInterpreterState_Get, PyObject,
    PyThreadState, _PyInterpreterState_GetEvalFrameFunc, _PyInterpreterState_SetEvalFrameFunc,
};
use pyo3::prelude::*;

#[pyfunction]
fn version() -> PyResult<String> {
    Ok(format!("{}.{}.{}", env!("CARGO_PKG_VERSION_MAJOR"), env!("CARGO_PKG_VERSION_MINOR"), env!("CARGO_PKG_VERSION_PATCH")))
}

static mut ORIGINAL_FRAME: Option<
    extern "C" fn(state: *mut PyThreadState, frame: *mut PyFrameObject, c: i32) -> *mut PyObject,
> = None;

extern "C" fn eval(state: *mut PyThreadState, frame: *mut PyFrameObject, c: i32) -> *mut PyObject {
    info!(target: "rupyjit", "eval()");

    unsafe {
        let f_code = frame.read().f_code.read().co_code;
        let is_bytes = PyBytes_Check(f_code);
        let n_bytes = PyBytes_Size(f_code);
        info!(target: "rupyjit", "is_bytes:{:?} n_bytes:{:?}", is_bytes, n_bytes);

        let code_buf = PyBytes_AsString(f_code);
        for i in 0..n_bytes {
            info!(target: "rupyjit", "code_buf[{}]:0x{:02x?}", i, *code_buf.offset(i as isize));
        }
    }

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
    info!(target: "rupyjit", "enable()");
    let state = unsafe { PyInterpreterState_Get() };
    unsafe { ORIGINAL_FRAME = Some(_PyInterpreterState_GetEvalFrameFunc(state)) };
    unsafe { _PyInterpreterState_SetEvalFrameFunc(state, eval) };
    Ok(())
}

/// A Python module implemented in Rust.
#[pymodule]
fn rupyjit(_py: Python, m: &PyModule) -> PyResult<()> {
    env_logger::init();
    m.add_function(wrap_pyfunction!(enable, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
