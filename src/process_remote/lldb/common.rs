#![allow(dead_code)]

use crate::process_remote::base_err;
use crate::Result;
use lldb::{lldb_addr_t, SBFileSpec, SBModule, SBProcess, SBTarget, SymbolType};
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
        self.process.unload_image(self.image_token).ok();
    }
}

pub fn load_image(process: &SBProcess, load_path: &std::path::Path) -> Result<LoadImageResult> {
    // on windows, we can find_module for modules we just loaded, so we use
    let target = process.target().ok_or(base_err("No target for process"))?;

    let path = load_path.to_str().ok_or(base_err("bad load_path"))?;
    let dylib = SBFileSpec::from_path(path, true);
    let image_token = process
        .load_image(&dylib)
        .map_err(|x| base_err(format!("failed to load image: {x:?}")))?;

    // not working on posix (at least macos)
    let Some(dylib) = target.find_module(&dylib) else {
        process.unload_image(image_token).ok();
        return Err(base_err("loaded module not found"));
    };

    fn find(module: &SBModule, target: &SBTarget, name: &str) -> Option<lldb_addr_t> {
        module
            .find_symbols(name, SymbolType::Any)
            .iter()
            .nth(0)
            .and_then(|x| x.symbol().start_address())
            .map(|x| x.load_address(target))
    }

    let Some(saver_save) = find(&dylib, &target, "CONSOLE_LOG_SAVER_SAVE") else {
        process.unload_image(image_token).ok();
        return Err(base_err("save symbol not found"));
    };

    let Some(free_mem) = find(&dylib, &target, "CONSOLE_LOG_SAVER_FREE_MEM") else {
        process.unload_image(image_token).ok();
        return Err(base_err("free_mem symbol not found"));
    };

    let Some(location) = find(&dylib, &target, "CONSOLE_LOG_SAVER_SAVED_LOCATION") else {
        process.unload_image(image_token).ok();
        return Err(base_err("location symbol not found"));
    };

    let process = process.clone();
    Ok(LoadImageResult {
        saver_save,
        free_mem,
        location,
        process,
        image_token,
    })
}

pub fn prepare_debug_server() -> crate::Result<Option<TempPath>> {
    Ok(None)
}
