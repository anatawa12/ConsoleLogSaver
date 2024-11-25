#![allow(dead_code)]

use crate::{SBFileSpecExt, SBProcessExt};
use lldb::{lldb_addr_t, FunctionNameType, SBFileSpec, SBProcess, SymbolType};
use std::convert::Infallible;
use tempfile::TempPath;

pub struct LoadImageResult {
    saver_save: lldb_addr_t,
    free_mem: lldb_addr_t,
    location: lldb_addr_t,
    process: SBProcess,
    image_token: u32,
}

impl LoadImageResult {
    pub fn saver_save(&self) -> lldb_addr_t {
        self.saver_save
    }

    pub fn free_mem(&self) -> lldb_addr_t {
        self.free_mem
    }

    pub fn location(&self) -> lldb_addr_t {
        self.location
    }

    pub fn unload(self) {
        self.process
            .unload_image(self.image_token)
            .expect("unloading image");
    }
}

fn load_image(
    process: &SBProcess,
    load_path: &std::path::Path,
) -> Result<LoadImageResult, Infallible> {
    // on windows, we can find_module for modules we just loaded, so we use
    let target = process.target().unwrap();

    let path = load_path.to_str().unwrap();
    let dylib = SBFileSpec::from_path(path);
    let image_token = process.load_image(&dylib).expect("loading image");

    // not working on posix (at least macos)
    let dylib = target.find_module(&dylib).expect("loaded dylib not found");

    let saver_save = dylib
        .find_functions("CONSOLE_LOG_SAVER_SAVE", FunctionNameType::AUTO.bits())
        .iter()
        .nth(0)
        .unwrap()
        .symbol()
        .start_address()
        .unwrap()
        .load_address(&target);
    let free_mem = dylib
        .find_functions("CONSOLE_LOG_SAVER_FREE_MEM", FunctionNameType::AUTO.bits())
        .iter()
        .nth(0)
        .unwrap()
        .symbol()
        .start_address()
        .unwrap()
        .load_address(&target);
    let location = dylib
        .find_symbols("CONSOLE_LOG_SAVER_SAVED_LOCATION", SymbolType::Data)
        .iter()
        .nth(0)
        .unwrap()
        .symbol()
        .start_address()
        .unwrap()
        .load_address(&target);

    let process = process.clone();
    Ok(LoadImageResult {
        saver_save,
        free_mem,
        location,
        process,
        image_token,
    })
}

pub fn prepare_debug_server() -> Option<TempPath> {
    None
}
