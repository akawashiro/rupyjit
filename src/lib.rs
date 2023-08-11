use log::info;
use pyo3::ffi::{
    PyBytes_AsString, PyBytes_Check, PyBytes_Size, PyFrameObject, PyInterpreterState_Get, PyObject,
    PyThreadState, PyTuple_GetItem, PyTuple_Size, _PyInterpreterState_GetEvalFrameFunc,
    _PyInterpreterState_SetEvalFrameFunc,
};
use pyo3::prelude::*;
use std::ffi::CStr;

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
            info!(target: "rupyjit", "code_vec[{}]:{:?}", i, code);
        } else {
            info!(target: "rupyjit", "code_vec[{}]:0x{:02x?}", i, c);
        }
    }
}

fn get_type(py_object: *mut PyObject) -> String {
    let c_str: &CStr = unsafe { CStr::from_ptr(py_object.read().ob_type.read().tp_name) };
    return c_str.to_str().unwrap().to_owned();
}

fn get_co_varnames(co_varnames: *mut PyObject) -> Vec<String> {
    info!("hoge");
    let n_co_varnames = unsafe { PyTuple_Size(co_varnames) };
    info!("{:?}", n_co_varnames);
    let ret = Vec::new();
    for i in 0..n_co_varnames {
        let t = unsafe { PyTuple_GetItem(co_varnames, i) };
        info!("{:?}", get_type(t));
        info!("{:?}", unsafe { PyBytes_Check(t) });
        let c_str = unsafe { CStr::from_ptr(PyBytes_AsString(t)) };
        info!("{:?}", c_str.to_str().unwrap().to_owned());
    }
    return ret;
}

extern "C" fn eval(state: *mut PyThreadState, frame: *mut PyFrameObject, c: i32) -> *mut PyObject {
    info!(target: "rupyjit", "eval()");

    unsafe {
        let f_code = frame.read().f_code.read().co_code;
        let is_bytes = PyBytes_Check(f_code);
        let n_bytes = PyBytes_Size(f_code);
        info!(target: "rupyjit", "is_bytes:{:?} n_bytes:{:?}", is_bytes, n_bytes);

        let code_buf = PyBytes_AsString(f_code);
        let mut code_vec = Vec::new();
        for i in 0..n_bytes {
            code_vec.push(*code_buf.offset(i as isize));
            info!(target: "rupyjit", "code_buf[{}]:0x{:02x?}", i, *code_buf.offset(i as isize));
        }
        show_code_vec(&code_vec);

        let co_varnames = frame.read().f_code.read().co_varnames;
        info!(target: "rupyjit", "{:?}", get_type(co_varnames));
        get_co_varnames(co_varnames);
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
