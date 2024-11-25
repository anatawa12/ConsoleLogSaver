mod common;
mod unix;

use super::ProcessRemoteError;
use lldb::{
    lldb_addr_t, lldb_pid_t, ByteOrder, SBAttachInfo, SBDebugger, SBError, SBExpressionOptions,
    SBFrame,
};
use std::io::Write;

#[cfg(not(unix))]
use common::load_image;
#[cfg(unix)]
use unix::load_image;

#[cfg(not(unix))]
use common::prepare_debug_server;
#[cfg(unix)]
use unix::prepare_debug_server;

pub fn get_buffer(pid: lldb_pid_t) -> Result<Vec<u8>, ProcessRemoteError> {
    SBDebugger::initialize();

    #[cfg(all(unix, not(feature = "external_debug_server")))]
    let _debugserver = prepare_debug_server();

    let attach_lib_dylib = {
        #[cfg(target_os = "windows")]
        let suffix = ".exe";
        #[cfg(target_os = "macos")]
        let suffix = ".dylib";
        #[cfg(target_os = "linux")]
        let suffix = ".so";

        let mut attach_lib_dylib = tempfile::Builder::new()
            .prefix("cls_attach_lib")
            .suffix(suffix)
            .tempfile()
            .expect("failed to create temporary file");

        attach_lib_dylib
            .write_all(include_bytes!(env!("CLS_ATTACH_LIB_PATH")))
            .expect("creating cls attach library failed");

        attach_lib_dylib
    };
    let attach_lib_dylib_path = attach_lib_dylib.into_temp_path();

    let debugger = SBDebugger::create(false);
    debugger.set_asynchronous(false);

    // reading symbol table took some time, we want to skip
    let target = debugger.create_target("", None, None, false).unwrap();

    let attach_info = SBAttachInfo::new_with_pid(pid);

    let process = target.attach(attach_info).unwrap();

    // I don't know wht but target.find_functions("SceneTracker::Update") don't work on windows
    // so we use different method
    let mut update = None;
    'modules: for module in target.modules() {
        if !module.filespec().filename().contains("Unity") {
            continue;
        }
        for symbol in module.symbols() {
            if symbol.name().contains("SceneTracker::Update(") {
                update = Some(
                    symbol
                        .start_address()
                        .expect("no start address for SceneTracker::Update"),
                );
                break 'modules;
            }
        }
    }

    let update = update.expect("SceneTracker::Update symbol not found");

    let breakpoint = target.breakpoint_create_by_sbaddress(update);
    breakpoint.set_oneshot(true);
    breakpoint.set_enabled(true);

    process.continue_execution().unwrap();
    target.delete_breakpoint(breakpoint.id());
    // now on breakpoint

    if target.byte_order() != current_byte_order() {
        return Err(ProcessRemoteError::ByteOrderMismatch);
    }

    if target.get_address_byte_size() as usize != size_of::<usize>() {
        return Err(ProcessRemoteError::PointerSizeMismatch);
    }

    let thread = process.selected_thread();
    let frame = thread.frames().nth(0).unwrap();

    let load_image = load_image(&process, attach_lib_dylib_path.as_ref()).expect("load_image");

    let saver_save = load_image.saver_save();
    let free_mem = load_image.free_mem();
    let location = load_image.location();

    eval_expr(
        &frame,
        &format!(
            r##"
        #!mini-llvm-expr 1
        const target_ptr ptr {saver_save}
        define_function_type void void_no_arg
        call _ void_no_arg target_ptr
        ret_void
        "##
        ),
    )
    .expect("calling saver");

    let mut pointer = 0usize;
    process
        .read_memory(
            location,
            bytemuck::cast_slice_mut(std::slice::from_mut(&mut pointer)),
        )
        .expect("reading pointer");
    let pointer = pointer as lldb_addr_t;

    let mut data_size = 0u64;
    process
        .read_memory(
            pointer,
            bytemuck::cast_slice_mut(std::slice::from_mut(&mut data_size)),
        )
        .expect("reading size memory");

    let mut buffer = vec![0u8; data_size as usize];
    process
        .read_memory(pointer + 8, &mut buffer)
        .expect("reading data memory");

    eval_expr(
        &frame,
        &format!(
            r##"
        #!mini-llvm-expr 1
        const target_ptr ptr {free_mem}
        define_function_type void void_no_arg
        call _ void_no_arg target_ptr
        ret_void
        "##
        ),
    )
    .expect("calling free_mem");

    load_image.unload();

    // I don't know why but detaching with synchronous and no resume
    // would freeze target process on detach after loading image.
    debugger.set_asynchronous(true);
    process.continue_execution().unwrap();

    process.detach().unwrap();

    SBDebugger::terminate();

    Ok(buffer)
}

fn eval_expr(frame: &SBFrame, expr: &str) -> Result<(), SBError> {
    let options = SBExpressionOptions::new();
    let result = frame.evaluate_expression(expr, &options);
    // 0x1001 is kNoResult, which is not an error
    // https://github.com/llvm/llvm-project/blob/d6e65a66095cc3c93ea78669bc41d0885780e8ea/lldb/include/lldb/Expression/UserExpression.h#L274
    if result
        .error()
        .map(|x| x.is_failure() && x.error() != 0x1001)
        .unwrap_or(false)
    {
        return Err(result.error().unwrap());
    }
    Ok(())
}

fn current_byte_order() -> ByteOrder {
    if cfg!(target_endian = "little") {
        ByteOrder::Little
    } else if cfg!(target_endian = "big") {
        ByteOrder::Big
    } else {
        ByteOrder::Invalid
    }
}
