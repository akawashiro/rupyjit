use log::info;
use pyo3::ffi::{
    PyBool_FromLong, PyBytes_AsString, PyBytes_Check, PyBytes_Size, PyDict_GetItem, PyDict_New,
    PyFrameObject, PyLong_AsLong, PyLong_FromLong, PyObject, PyObject_Call, PyThreadState,
    PyTuple_GetItem, PyTuple_New, PyTuple_SetItem, Py_IsTrue,
};
use std::io::{self, Write};

#[path = "bytecode.rs"]
mod bytecode;
use bytecode::Bytecode;
#[path = "pyutils.rs"]
mod pyutils;
use pyutils::dump_frame_info;

extern crate libc;
use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter};
use libc::{c_int, c_void, size_t, PROT_EXEC, PROT_READ, PROT_WRITE};
use std::alloc::{alloc, Layout};

extern "C" {
    fn mprotect(addr: *const c_void, len: size_t, prot: c_int) -> c_int;
}

macro_rules! jit_log {
    ($x:expr) => {
        let _ = io::stdout().write_all(
            (file!().to_owned() + ":" + &line!().to_string() + " jit_log " + $x + "\n").as_bytes(),
        );
        let _ = io::stdout().flush();
    };
}

fn write_nop(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x90 };
    index + 1
}

#[allow(dead_code)]
fn write_software_breakpoint(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0xcc };
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

fn write_cmp_rax_0(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x48 };
    unsafe { *(buf.add(index + 1)) = 0x83 };
    unsafe { *(buf.add(index + 2)) = 0xf8 };
    unsafe { *(buf.add(index + 3)) = 0x00 };
    index + 4
}

fn write_je(buf: *mut u8, index: usize, offset: i32) -> usize {
    unsafe { *(buf.add(index)) = 0x0f };
    unsafe { *(buf.add(index + 1)) = 0x84 };
    for i in 0..4 {
        unsafe { *(buf.add(i + index + 2)) = (offset >> (i * 8)) as u8 };
    }
    index + 6
}

fn write_endbr64(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0xf3 };
    unsafe { *(buf.add(index + 1)) = 0x0f };
    unsafe { *(buf.add(index + 2)) = 0x1e };
    unsafe { *(buf.add(index + 3)) = 0xfa };
    index + 4
}

fn write_push_rbp(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x55 };
    index + 1
}

#[allow(dead_code)]
fn write_mov_rsp_to_rbp(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x48 };
    unsafe { *(buf.add(index + 1)) = 0x89 };
    unsafe { *(buf.add(index + 2)) = 0xe5 };
    index + 3
}

fn write_push_rax(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x50 };
    index + 1
}

#[allow(dead_code)]
fn write_push_dummy_value(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x68 };
    unsafe { *(buf.add(index + 1)) = 0xaa };
    unsafe { *(buf.add(index + 2)) = 0xbb };
    unsafe { *(buf.add(index + 3)) = 0xcc };
    unsafe { *(buf.add(index + 4)) = 0xdd };
    index + 5
}

#[allow(dead_code)]
fn write_push_r11(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x41 };
    unsafe { *(buf.add(index + 1)) = 0x53 };
    index + 2
}

fn write_pop_rbp(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x5d };
    index + 1
}

fn write_pop_rax(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x58 };
    index + 1
}

#[allow(dead_code)]
fn write_pop_r11(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x41 };
    unsafe { *(buf.add(index + 1)) = 0x5b };
    index + 2
}

fn write_pop_rdi(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x5f };
    index + 1
}

fn write_pop_rsi(buf: *mut u8, index: usize) -> usize {
    unsafe { *(buf.add(index)) = 0x5e };
    index + 1
}

fn add_py_longs(a: *mut PyObject, b: *mut PyObject) -> *mut PyObject {
    unsafe {
        let a = PyLong_AsLong(a);
        let b = PyLong_AsLong(b);
        let c = a + b;
        PyLong_FromLong(c)
    }
}

fn sub_py_longs(a: *mut PyObject, b: *mut PyObject) -> *mut PyObject {
    unsafe {
        let a = PyLong_AsLong(a);
        let b = PyLong_AsLong(b);
        let c = a - b;
        PyLong_FromLong(c)
    }
}

fn compare_py_longs(a: *mut PyObject, b: *mut PyObject) -> *mut PyObject {
    unsafe {
        let a = PyLong_AsLong(a);
        let b = PyLong_AsLong(b);
        PyBool_FromLong(if b - a < 0 { 1 } else { 0 })
    }
}

fn check_py_bool(a: *mut PyObject) -> i64 {
    let b = unsafe { Py_IsTrue(a) };
    jit_log!(&format!("check_py_bool:{}", b));
    if b == 1 {
        1
    } else {
        0
    }
}

fn call_callable(callable: *mut PyObject, arg: *mut PyObject) -> *mut PyObject {
    jit_log!("call_callable");
    let args = unsafe { PyTuple_New(1) };
    unsafe { PyTuple_SetItem(args, 0, arg) };
    let kwargs = unsafe { PyDict_New() };
    let r: *mut PyObject = unsafe { PyObject_Call(callable, args, kwargs) };
    jit_log!("call_callable");
    r
}

pub fn compile_and_exec_jit_code(
    state: *mut PyThreadState,
    frame: *mut PyFrameObject,
    c: i32,
) -> Option<*mut PyObject> {
    info!("compile_and_exec_jit_code");
    dump_frame_info(state, frame, c);

    const CODE_AREA_SIZE: usize = 1024;
    const PAGE_SIZE: usize = 4096;

    let layout = Layout::from_size_align(CODE_AREA_SIZE, PAGE_SIZE).unwrap();
    let p_start = unsafe { alloc(layout) };
    let mem = unsafe {
        mprotect(
            p_start as *const c_void,
            CODE_AREA_SIZE,
            PROT_READ | PROT_WRITE | PROT_EXEC,
        )
    };
    assert_eq!(mem, 0);
    let mut offset = 0;

    // Write endbr64
    offset = write_endbr64(p_start, offset);

    // Write push rbp
    offset = write_push_rbp(p_start, offset);

    // Compile
    {
        let f_code = unsafe { frame.read().f_code.read().co_code };
        let is_bytes = unsafe { PyBytes_Check(f_code) };
        let n_bytes = unsafe { PyBytes_Size(f_code) };
        info!("is_bytes:{:?} n_bytes:{:?}", is_bytes, n_bytes);

        let code_buf = unsafe { PyBytes_AsString(f_code) };
        let mut code_vec: Vec<u8> = Vec::new();
        for i in 0..n_bytes {
            unsafe { code_vec.push(*code_buf.offset(i as isize) as u8) };
        }

        // Show code
        for i in (0..n_bytes).step_by(2) {
            let code: Bytecode =
                unsafe { num::FromPrimitive::from_u8(*code_buf.offset(i) as u8).unwrap() };
            let arg: i8 = unsafe { *code_buf.offset(i as isize + 1) };
            info!("code_vec[{}]:{:?}, 0x{:02x?}", i, code, arg);
        }

        // Hack to realize relative jump in Python bytecode easily. All byte code is translated to
        // x86_64 code with bytes_per_code bytes.
        let bytes_per_code: u32 = 24;

        // Compile
        for i in (0..n_bytes).step_by(2) {
            let code: Bytecode =
                unsafe { num::FromPrimitive::from_u8(*code_buf.offset(i) as u8).unwrap() };
            let arg: i8 = unsafe { *code_buf.offset(i as isize + 1) };
            let start_offset = offset;
            match code {
                Bytecode::LoadFast => {
                    let l = unsafe { frame.read().f_localsplus[arg as usize] };
                    // MOV RAX, l
                    offset = write_mov_rax(p_start, offset, l as u64);
                    // PUSH RAX
                    offset = write_push_rax(p_start, offset);
                }
                Bytecode::ReturnValue => {
                    // POP RAX
                    offset = write_pop_rax(p_start, offset);
                    // POP RBP
                    offset = write_pop_rbp(p_start, offset);
                    // RET
                    offset = write_ret(p_start, offset);
                }
                Bytecode::BinaryAdd => {
                    // POP RDI
                    offset = write_pop_rdi(p_start, offset);
                    // POP RSI
                    offset = write_pop_rsi(p_start, offset);

                    // MOV $RAX, add_py_longs
                    offset = write_mov_rax(p_start, offset, add_py_longs as u64);
                    // CALL $RAX
                    offset = write_call_rax(p_start, offset);

                    // PUSH RAX
                    offset = write_push_rax(p_start, offset);
                }
                Bytecode::BinarySubtract => {
                    // POP RSI
                    offset = write_pop_rsi(p_start, offset);
                    // POP RDI
                    offset = write_pop_rdi(p_start, offset);

                    // MOV $RAX, sub_py_longs
                    offset = write_mov_rax(p_start, offset, sub_py_longs as u64);
                    // CALL $RAX
                    offset = write_call_rax(p_start, offset);

                    // PUSH RAX
                    offset = write_push_rax(p_start, offset);
                }
                Bytecode::CompareOp => {
                    match arg {
                        0 => {
                            // POP RDI
                            offset = write_pop_rdi(p_start, offset);
                            // POP RSI
                            offset = write_pop_rsi(p_start, offset);

                            // MOV $RAX, sub_py_longs
                            offset = write_mov_rax(p_start, offset, compare_py_longs as u64);
                            // CALL $RAX
                            offset = write_call_rax(p_start, offset);

                            // PUSH RAX
                            offset = write_push_rax(p_start, offset);
                        }
                        _ => {
                            info!("Unknown code:{:?}", code);
                            info!("Fallback to the Python interpreter");
                            return None;
                        }
                    }
                }
                Bytecode::LoadConst => {
                    let const_table = unsafe { frame.read().f_code.read().co_consts };
                    let const_object = unsafe { PyTuple_GetItem(const_table, arg as isize) };

                    // MOV RAX, const_object
                    offset = write_mov_rax(p_start, offset, const_object as u64);
                    // PUSH RAX
                    offset = write_push_rax(p_start, offset);
                }
                Bytecode::PopJumpIfFalse => {
                    // POP RDI
                    offset = write_pop_rdi(p_start, offset);
                    // MOV $RAX, check_py_bool
                    offset = write_mov_rax(p_start, offset, check_py_bool as u64);
                    // CALL $RAX
                    offset = write_call_rax(p_start, offset);

                    // Now RAX is the result of check_py_bool
                    // 1 is true, 0 is false
                    // CMP RAX, 0
                    offset = write_cmp_rax_0(p_start, offset);
                    // JE
                    offset = write_je(p_start, offset, ((arg / 2) as u32 * bytes_per_code) as i32);
                }
                Bytecode::LoadGlobal => {
                    let name = unsafe {
                        PyTuple_GetItem(frame.read().f_code.read().co_names, arg as isize)
                    };
                    let globals = unsafe { frame.read().f_globals };
                    let value = unsafe { PyDict_GetItem(globals, name) };
                    // MOV RAX, value
                    offset = write_mov_rax(p_start, offset, value as u64);
                    // PUSH RAX
                    offset = write_push_rax(p_start, offset);
                }
                Bytecode::CallFunction => {
                    assert_eq!(arg, 1, "Only support 1 argument function");
                    // POP RDI
                    offset = write_pop_rdi(p_start, offset);
                    // POP RSI
                    offset = write_pop_rsi(p_start, offset);
                    // MOV $RAX, call_callable
                    offset = write_mov_rax(p_start, offset, call_callable as u64);

                    // CALL $RAX
                    offset = write_call_rax(p_start, offset);
                    // PUSH RAX
                    offset = write_push_rax(p_start, offset);
                }
                _ => {
                    info!("Unknown code:{:?}", code);
                    info!("Fallback to the Python interpreter");
                    return None;
                }
            }
            while (offset - start_offset) < bytes_per_code.try_into().unwrap() {
                offset = write_nop(p_start, offset);
            }
            assert_eq!(offset - start_offset, bytes_per_code.try_into().unwrap());
        }
        if std::env::var("RUST_LOG") == Result::Ok(String::from("info"))
            || std::env::var("RUST_LOG") == Result::Ok(String::from("debug"))
        {
            log_disasm(p_start, offset, code_vec, bytes_per_code);
        }
    }
    let code: fn() -> *mut PyObject = unsafe { std::mem::transmute(p_start) };

    info!("Jump to code:{:x?}", code);
    let retval = code();
    info!("Return from code:{:x?} retval:{:x?}", code, retval);
    return Some(retval);
}

fn log_disasm(code: *const u8, code_size: usize, py_code_vec: Vec<u8>, bytes_per_code: u32) {
    assert_eq!(py_code_vec.len() % 2, 0);
    assert!(
        code_size >= py_code_vec.len() / 2 * bytes_per_code as usize,
        "code_size:{:?} py_code_vec.len():{:?} bytes_per_code:{:?}",
        code_size,
        py_code_vec.len(),
        bytes_per_code
    );

    let mut code_vec: Vec<u8> = Vec::new();
    for i in 0..code_size {
        code_vec.push(unsafe { *code.offset(i as isize) });
    }
    let bytes = code_vec.as_slice();
    let bitness = 64;
    let rip = code as u64;
    const HEXBYTES_COLUMN_BYTE_LENGTH: usize = 10;
    let mut decoder = Decoder::with_ip(bitness, code_vec.as_slice(), rip, DecoderOptions::NONE);

    // Formatters: Masm*, Nasm*, Gas* (AT&T) and Intel* (XED).
    // For fastest code, see `SpecializedFormatter` which is ~3.3x faster. Use it if formatting
    // speed is more important than being able to re-assemble formatted instructions.
    let mut formatter = IntelFormatter::new();

    // Change some options, there are many more
    formatter.options_mut().set_digit_separator("`");
    formatter.options_mut().set_first_operand_char_index(10);

    // String implements FormatterOutput
    let mut output = String::new();

    // Initialize this outside the loop because decode_out() writes to every field
    let mut instruction = Instruction::default();

    // The decoder also implements Iterator/IntoIterator so you could use a for loop:
    //      for instruction in &mut decoder { /* ... */ }
    // or collect():
    //      let instructions: Vec<_> = decoder.into_iter().collect();
    // but can_decode()/decode_out() is a little faster:
    while decoder.can_decode() {
        // 5 is a magic number of endbr64 + push rbp.
        if decoder.position() % bytes_per_code as usize == 5 {
            let code: Bytecode = num::FromPrimitive::from_u8(
                py_code_vec[(decoder.position() - 5) / bytes_per_code as usize * 2],
            )
            .unwrap();
            let arg = py_code_vec[(decoder.position() - 5) / bytes_per_code as usize * 2 + 1];
            println!("; {:?}, 0x{:02x?}", code, arg);
        }
        // There's also a decode() method that returns an instruction but that also
        // means it copies an instruction (40 bytes):
        //     instruction = decoder.decode();
        decoder.decode_out(&mut instruction);

        // Format the instruction ("disassemble" it)
        output.clear();
        formatter.format(&instruction, &mut output);

        // Eg. "00007FFAC46ACDB2 488DAC2400FFFFFF     lea       rbp,[rsp-100h]"
        print!("{:016X} ", instruction.ip());
        let start_index = (instruction.ip() - rip) as usize;
        let instr_bytes = &bytes[start_index..start_index + instruction.len()];
        for b in instr_bytes.iter() {
            print!("{:02X}", b);
        }
        if instr_bytes.len() < HEXBYTES_COLUMN_BYTE_LENGTH {
            for _ in 0..HEXBYTES_COLUMN_BYTE_LENGTH - instr_bytes.len() {
                print!("  ");
            }
        }
        println!(" {}", output);
    }
}
