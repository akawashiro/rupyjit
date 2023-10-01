use log::info;
use pyo3::ffi::{
    PyBytes_AsString, PyBytes_Check, PyBytes_Size, PyDict_Check, PyDict_Keys, PyFrameObject,
    PyList_GetItem, PyList_Size, PyLongObject, PyLong_AsLong, PyLong_Check, PyObject,
    PyThreadState, PyTuple_Check, PyTuple_GetItem, PyTuple_Size, PyUnicode_AsUTF8, PyUnicode_Check,
};
use std::ffi::CStr;
#[path = "bytecode.rs"]
mod bytecode;
use bytecode::Bytecode;

fn c_bytes_to_string(b: *const i8) -> String {
    let c_str: &CStr = unsafe { CStr::from_ptr(b) };
    return c_str.to_str().unwrap().to_owned();
}

fn show_code_vec(code_vec: &Vec<u8>) {
    for (i, c) in code_vec.iter().enumerate() {
        if i % 2 == 0 {
            let code: Option<Bytecode> = num::FromPrimitive::from_u8(*c);
            if !code.is_none() {
                info!("code_vec[{}]:{:?}({:?})", i, code.unwrap(), *c as u8);
            } else {
                info!("code_vec[{}]:{:?}", i, *c as u8);
                panic!("Unknown code");
            }
        } else {
            info!("code_vec[{}]:0x{:02x?}", i, c);
        }
    }
}

fn get_type(py_object: *mut PyObject) -> String {
    return unsafe { c_bytes_to_string(py_object.read().ob_type.read().tp_name) };
}

fn str_to_string(s: *mut PyObject) -> String {
    assert_eq!(unsafe { PyUnicode_Check(s) }, 1);
    let u = unsafe { PyUnicode_AsUTF8(s) };
    let s = c_bytes_to_string(u);
    return s;
}

fn get_co_varnames(co_varnames: *mut PyObject) -> Vec<String> {
    assert_eq!(unsafe { PyTuple_Check(co_varnames) }, 1);
    let n_co_varnames = unsafe { PyTuple_Size(co_varnames) };
    let mut ret = Vec::new();
    for i in 0..n_co_varnames {
        let t = unsafe { PyTuple_GetItem(co_varnames, i) };
        let u = unsafe { PyUnicode_AsUTF8(t) };
        let s = c_bytes_to_string(u);
        ret.push(s);
    }
    return ret;
}

fn get_dict_keys(d: *mut PyObject) -> Vec<String> {
    assert_eq!(unsafe { PyDict_Check(d) }, 1);
    let ks = unsafe { PyDict_Keys(d) };
    let n = unsafe { PyList_Size(ks) };
    let mut ret = Vec::new();
    for i in 0..n {
        let k = unsafe { PyList_GetItem(ks, i) };
        let s = str_to_string(k);
        ret.push(s);
    }
    return ret;
}

fn get_jit_key(frame: *mut PyFrameObject) -> String {
    let mut fn_name = unsafe { str_to_string(frame.read().f_code.read().co_name) };
    let co_argcounts = unsafe { frame.read().f_code.read().co_argcount };
    for i in 0..co_argcounts {
        let l = unsafe { frame.read().f_localsplus[i as usize] };
        let t = get_type(l);
        fn_name.push_str(&format!("_{}", t));
    }
    return fn_name;
}

pub fn dump_frame_info(_state: *mut PyThreadState, frame: *mut PyFrameObject, _c: i32) {
    info!("dump_frame_info");
    unsafe {
        let f_code = frame.read().f_code.read().co_code;
        let is_bytes = PyBytes_Check(f_code);
        let n_bytes = PyBytes_Size(f_code);
        info!("is_bytes:{:?} n_bytes:{:?}", is_bytes, n_bytes);

        let code_buf = PyBytes_AsString(f_code);
        let mut code_vec: Vec<u8> = Vec::new();
        for i in 0..n_bytes {
            code_vec.push(*code_buf.offset(i as isize) as u8);
            // info!("code_buf[{}]:0x{:02x?}", i, *code_buf.offset(i as isize));
        }
        show_code_vec(&code_vec);

        let co_varnames = frame.read().f_code.read().co_varnames;
        let co_varnames = get_co_varnames(co_varnames);
        info!("co_varnames={:?}", co_varnames);

        let f_globals = frame.read().f_globals;
        info!("f_globals={:?}", get_dict_keys(f_globals));
        let f_locals = frame.read().f_locals;
        info!("f_locals={:?}", f_locals);

        let co_nlocals = frame.read().f_code.read().co_nlocals;
        info!("co_nlocals={:?}", co_nlocals);

        let co_consts = frame.read().f_code.read().co_consts;
        info!("co_consts={:?}", co_consts);

        let co_argcounts = frame.read().f_code.read().co_argcount;
        info!("co_argcounts={:?}", co_argcounts);

        info!("frame.read().f_stackdepth={:?}", frame.read().f_stackdepth);
        info!("frame.read().f_stacktop={:?}", frame.read().f_valuestack);
        info!(
            "frame.read().f_localsplus[0]={:?}",
            frame.read().f_localsplus[0]
        );

        for i in 0..co_argcounts {
            let l = frame.read().f_localsplus[i as usize];
            // let l = f_localsplus_head.offset(1 as isize);
            info!("l={:?}", l);
            // info!("l={:?}", *l);
            info!("size of PyObject = {:?}", std::mem::size_of::<PyObject>());
            info!(
                "size of PyLongObject = {:?}",
                std::mem::size_of::<PyLongObject>()
            );
            // info!("Py_SIZE(l) = {:?}", Py_SIZE(l));
            info!("get_type(l)={:?}", get_type(l));
            if PyLong_Check(l) == 1 {
                info!("PyLong_AsLong(l)={:?}", PyLong_AsLong(l));
            }
        }

        info!(
            "PyUnicode_Check(frame.read().f_code.read().co_name)={:?}",
            PyUnicode_Check(frame.read().f_code.read().co_name)
        );
        info!(
            "str_to_string(frame.read().f_code.read().co_name)={:?}",
            str_to_string(frame.read().f_code.read().co_name)
        );
    }
    info!("get_jit_key(frame)={:?}", get_jit_key(frame));
}
