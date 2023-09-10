use log::info;
use pyo3::ffi::{
    PyBytes_AsString, PyBytes_Check, PyBytes_Size, PyDict_Check, PyDict_Keys, PyFrameObject,
    PyInterpreterState_Get, PyList_GetItem, PyList_Size, PyLongObject, PyLong_AsLong, PyLong_Check,
    PyObject, PyThreadState, PyTuple_Check, PyTuple_GetItem, PyTuple_Size, PyUnicode_AsUTF8,
    PyUnicode_Check, _PyInterpreterState_GetEvalFrameFunc, _PyInterpreterState_SetEvalFrameFunc,
};
use std::ffi::CStr;
use std::io::{self, Write};

#[path = "bytecode.rs"]
mod bytecode;
use bytecode::Bytecode;

extern crate libc;
use libc::{c_int, c_void, size_t, PROT_EXEC, PROT_READ, PROT_WRITE};
use std::alloc::{alloc, dealloc, Layout};

extern "C" {
    fn mprotect(addr: *const c_void, len: size_t, prot: c_int) -> c_int;
}

macro_rules! jit_log {
    ($x:expr) => {
        io::stdout().write_all(
            (file!().to_owned() + ":" + &line!().to_string() + " jit_log " + $x + "\n").as_bytes(),
        );
        io::stdout().flush();
    };
}

fn foo() {
    jit_log!("foo DAYO!");
}

fn add_jit(a: *mut PyObject, b: *mut PyObject) {
    unsafe {
        // let a = PyLong_AsLong(a);
        // let b = PyLong_AsLong(b);
        // let c = a + b;
        jit_log!(&format!("PyLong_Check(a):{}", PyLong_Check(a)).to_string());
        jit_log!(&format!("PyLong_Check(b):{}", PyLong_Check(b)).to_string());
    }
}

fn write_nop(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x90 };
    index + 1
}

fn write_mov_rax(buf: *mut u8, index: usize, value: u64) -> usize {
    unsafe { *(buf.add(index)) = 0x48 };
    unsafe { *(buf.add(index + 1)) = 0xb8 };
    for i in 0..8 {
        unsafe { *(buf.add(i + index + 2)) = (value >> (i * 8)) as u8 };
    }
    index + 10
}

fn write_call_rax(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0xff };
    unsafe { *(buf.add(index + 1)) = 0xd0 };
    index + 2
}

fn write_ret(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0xc3 };
    index + 1
}

fn write_u64_to_bytes(buf: *mut u8, index: usize, value: u64) -> usize {
    for i in 0..8 {
        unsafe { *(buf.add(i + index)) = (value >> (i * 8)) as u8 };
    }
    index + 8
}

pub fn exec_jit_code(state: *mut PyThreadState, frame: *mut PyFrameObject, c: i32) {
    info!("exec_jit_code");

    const CODE_AREA_SIZE: usize = 1024;
    const PAGE_SIZE: usize = 4096;

    unsafe {
        let layout = Layout::from_size_align(CODE_AREA_SIZE, PAGE_SIZE).unwrap();
        let p_start = alloc(layout);
        let foo_addr = foo as *const fn() as u64;
        let rel_addr = (foo as *const fn() as usize) - (p_start as usize);
        info!(
            "p_start:0x{:x?} foo:0x{:x?} rel_addr:0x{:x?}",
            p_start, foo as *const fn(), rel_addr
        );

        let mem = mprotect(
            p_start as *const c_void,
            CODE_AREA_SIZE,
            PROT_READ | PROT_WRITE | PROT_EXEC,
        );
        assert_eq!(mem, 0);
        let mut offset = 0;

        offset = write_nop(p_start, offset);
        offset = write_nop(p_start, offset);

        // MOV $RAX, foo_addr
        offset = write_mov_rax(p_start, offset, foo_addr);

        // CALL $RAX
        offset = write_call_rax(p_start, offset);

        // Set return value
        offset = write_mov_rax(p_start, offset, 0xdeadbeefdeadbeef);

        // Fill with NOP
        while offset < CODE_AREA_SIZE - 1 {
            offset = write_nop(p_start, offset);
        }

        // RET
        let _ = write_ret(p_start, offset);
        let code: fn() -> u64 = std::mem::transmute(p_start);

        info!("Jump to code:0x{:x?}", code);
        let retval = code();
        info!("Return from code:0x{:x?} retval:0x{:x?}", code, retval);
    }
}

pub fn show_code_vec(code_vec: &Vec<i8>) {
    for (i, c) in code_vec.iter().enumerate() {
        if i % 2 == 0 {
            let code: Bytecode = num::FromPrimitive::from_i8(*c).unwrap();
            info!("code_vec[{}]:{:?}", i, code);
        } else {
            info!("code_vec[{}]:0x{:02x?}", i, c);
        }
    }
}

fn c_bytes_to_string(b: *const i8) -> String {
    let c_str: &CStr = unsafe { CStr::from_ptr(b) };
    return c_str.to_str().unwrap().to_owned();
}

pub fn get_type(py_object: *mut PyObject) -> String {
    return unsafe { c_bytes_to_string(py_object.read().ob_type.read().tp_name) };
}

pub fn str_to_string(s: *mut PyObject) -> String {
    assert_eq!(unsafe { PyUnicode_Check(s) }, 1);
    let u = unsafe { PyUnicode_AsUTF8(s) };
    let s = c_bytes_to_string(u);
    return s;
}

pub fn get_co_varnames(co_varnames: *mut PyObject) -> Vec<String> {
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

pub fn get_dict_keys(d: *mut PyObject) -> Vec<String> {
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

pub fn get_jit_key(frame: *mut PyFrameObject) -> String {
    let mut fn_name = unsafe { str_to_string(frame.read().f_code.read().co_name) };
    let co_argcounts = unsafe { frame.read().f_code.read().co_argcount };
    for i in 0..co_argcounts {
        let l = unsafe { frame.read().f_localsplus[i as usize] };
        let t = get_type(l);
        fn_name.push_str(&format!("_{}", t));
    }
    return fn_name;
}

pub fn dump_frame_info(state: *mut PyThreadState, frame: *mut PyFrameObject, c: i32) {
    info!("dump_frame_info");
    unsafe {
        let f_code = frame.read().f_code.read().co_code;
        let is_bytes = PyBytes_Check(f_code);
        let n_bytes = PyBytes_Size(f_code);
        info!("is_bytes:{:?} n_bytes:{:?}", is_bytes, n_bytes);

        let code_buf = PyBytes_AsString(f_code);
        let mut code_vec = Vec::new();
        for i in 0..n_bytes {
            code_vec.push(*code_buf.offset(i as isize));
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
