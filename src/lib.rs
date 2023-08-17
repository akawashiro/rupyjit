use chrono;
use log::{info, LevelFilter};
use pyo3::ffi::{
    PyBytes_AsString, PyBytes_Check, PyBytes_Size, PyDict_Check, PyDict_Keys, PyFrameObject,
    PyInterpreterState_Get, PyList_GetItem, PyList_Size, PyLongObject, PyLong_AsLong, PyLong_Check,
    PyObject, PyThreadState, PyTuple_Check, PyTuple_GetItem, PyTuple_Size, PyUnicode_AsUTF8,
    PyUnicode_Check, Py_SIZE, _PyInterpreterState_GetEvalFrameFunc,
    _PyInterpreterState_SetEvalFrameFunc,
};
use pyo3::prelude::*;
use std::ffi::CStr;
use std::io::Write;

extern crate num;
#[macro_use]
extern crate num_derive;

#[pyfunction]
fn version() -> PyResult<String> {
    Ok(format!(
        "{}.{}.{}",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH")
    ))
}

static mut ORIGINAL_FRAME: Option<
    extern "C" fn(state: *mut PyThreadState, frame: *mut PyFrameObject, c: i32) -> *mut PyObject,
> = None;

#[derive(Debug, FromPrimitive)]
enum Bytecode {
    Cache = 0,
    PopTop = 1,
    PushNull = 2,
    InterpreterExit = 3,
    EndFor = 4,
    EndSend = 5,
    ToBool = 6,
    Nop = 9,
    UnaryNegative = 11,
    UnaryNot = 12,
    UnaryInvert = 15,
    ExitInitCheck = 16,
    Reserved = 17,
    BinaryAdd = 0x17,
    MakeFunction = 24,
    BinarySubscr = 25,
    BinarySlice = 26,
    StoreSlice = 27,
    GetLen = 30,
    MatchMapping = 31,
    MatchSequence = 32,
    MatchKeys = 33,
    PushExcInfo = 35,
    CheckExcMatch = 36,
    CheckEgMatch = 37,
    FormatSimple = 40,
    FormatWithSpec = 41,
    WithExceptStart = 49,
    GetAiter = 50,
    GetAnext = 51,
    BeforeAsyncWith = 52,
    BeforeWith = 53,
    EndAsyncFor = 54,
    CleanupThrow = 55,
    StoreSubscr = 60,
    DeleteSubscr = 61,
    GetIter = 68,
    GetYieldFromIter = 69,
    LoadBuildClass = 71,
    LoadAssertionError = 74,
    ReturnGenerator = 75,
    ReturnValue = 83,
    SetupAnnotations = 85,
    LoadLocals = 87,
    PopExcept = 89,
    StoreName = 90,
    DeleteName = 91,
    UnpackSequence = 92,
    ForIter = 93,
    UnpackEx = 94,
    StoreAttr = 95,
    DeleteAttr = 96,
    StoreGlobal = 97,
    DeleteGlobal = 98,
    Swap = 99,
    LoadConst = 100,
    LoadName = 101,
    BuildTuple = 102,
    BuildList = 103,
    BuildSet = 104,
    BuildMap = 105,
    LoadAttr = 106,
    CompareOp = 107,
    ImportName = 108,
    ImportFrom = 109,
    JumpForward = 110,
    PopJumpIfFalse = 114,
    PopJumpIfTrue = 115,
    LoadGlobal = 116,
    IsOp = 117,
    ContainsOp = 118,
    Reraise = 119,
    Copy = 120,
    ReturnConst = 121,
    BinaryOp = 122,
    Send = 123,
    LoadFast = 124,
    StoreFast = 125,
    DeleteFast = 126,
    LoadFastCheck = 127,
    PopJumpIfNotNone = 128,
    PopJumpIfNone = 129,
    RaiseVarargs = 130,
    GetAwaitable = 131,
    BuildSlice = 133,
    JumpBackwardNoInterrupt = 134,
    MakeCell = 135,
    LoadDeref = 137,
    StoreDeref = 138,
    DeleteDeref = 139,
    JumpBackward = 140,
    LoadSuperAttr = 141,
    CallFunctionEx = 142,
    LoadFastAndClear = 143,
    ExtendedArg = 144,
    ListAppend = 145,
    SetAdd = 146,
    MapAdd = 147,
    CopyFreeVars = 149,
    YieldValue = 150,
    Resume = 151,
    MatchClass = 152,
    BuildConstKeyMap = 156,
    BuildString = 157,
    ConvertValue = 158,
    ListExtend = 162,
    SetUpdate = 163,
    DictMerge = 164,
    DictUpdate = 165,
    LoadFastLoadFast = 168,
    StoreFastLoadFast = 169,
    StoreFastStoreFast = 170,
    Call = 171,
    KwNames = 172,
    CallIntrinsic1 = 173,
    CallIntrinsic2 = 174,
    LoadFromDictOrGlobals = 175,
    LoadFromDictOrDeref = 176,
    SetFunctionAttribute = 177,
    EnterExecutor = 230,
    InstrumentedLoadSuperAttr = 237,
    InstrumentedPopJumpIfNone = 238,
    InstrumentedPopJumpIfNotNone = 239,
    InstrumentedResume = 240,
    InstrumentedCall = 241,
    InstrumentedReturnValue = 242,
    InstrumentedYieldValue = 243,
    InstrumentedCallFunctionEx = 244,
    InstrumentedJumpForward = 245,
    InstrumentedJumpBackward = 246,
    InstrumentedReturnConst = 247,
    InstrumentedForIter = 248,
    InstrumentedPopJumpIfFalse = 249,
    InstrumentedPopJumpIfTrue = 250,
    InstrumentedEndFor = 251,
    InstrumentedEndSend = 252,
    InstrumentedInstruction = 253,
    InstrumentedLine = 254,
    SetupFinally = 256,
    SetupCleanup = 257,
    SetupWith = 258,
    PopBlock = 259,
    Jump = 260,
    JumpNoInterrupt = 261,
    LoadMethod = 262,
    LoadSuperMethod = 263,
    LoadZeroSuperMethod = 264,
    LoadZeroSuperAttr = 265,
    StoreFastMaybeNull = 266,
    LoadClosure = 267,
}

fn show_code_vec(code_vec: &Vec<i8>) {
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

extern "C" fn eval(state: *mut PyThreadState, frame: *mut PyFrameObject, c: i32) -> *mut PyObject {
    info!("eval()");

    unsafe {
        let f_code = frame.read().f_code.read().co_code;
        let is_bytes = PyBytes_Check(f_code);
        let n_bytes = PyBytes_Size(f_code);
        info!("is_bytes:{:?} n_bytes:{:?}", is_bytes, n_bytes);

        let code_buf = PyBytes_AsString(f_code);
        let mut code_vec = Vec::new();
        for i in 0..n_bytes {
            code_vec.push(*code_buf.offset(i as isize));
            info!("code_buf[{}]:0x{:02x?}", i, *code_buf.offset(i as isize));
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

        // let f_localsplus = frame.read().f_valuestack;

        for i in 0..co_argcounts {
            let l = frame.read().f_localsplus[i as usize];
            // let l = f_localsplus.offset(2 as isize);
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

    info!("hello world");
    m.add_function(wrap_pyfunction!(enable, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
