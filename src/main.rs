mod cls_file;
mod process_remote;

use crate::cls_file::{ClsFileBuilder, ClsHeadingBuilder};
use byteorder::{NativeEndian, ReadBytesExt};
use lldb::{
    lldb_addr_t, lldb_offset_t, lldb_pid_t, ByteOrder, SBAddress,
    SBData, SBError, SBFileSpec, 
    SBModule, SBModuleSpec, SBProcess, SBSection, SBSymbol, SBTarget, SBValue,
};
use serde::Deserialize;
use std::env::args;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

fn main() {
    let mut args = args();

    let unity_pid = args
        .nth(1)
        .expect("please specify pid")
        .parse::<lldb_pid_t>()
        .expect("Failed to parse unity pid");

    let buffer = process_remote::get_buffer(unity_pid).expect("Failed to get buffer");

    let mut reader = TransferDataReader::new(buffer);

    let version: i32 = reader.read_i32();
    if version == 1 {
        let mut cls_file_builder = ClsFileBuilder::new();
        cls_file_builder.add_header(
            "Vendor",
            concat!(
                "ConsoleLogSaver/",
                env!("CARGO_PKG_VERSION"),
                " (CLS-LLDB-RS)"
            ),
        );

        let unity_version = reader.read_string();
        cls_file_builder.add_header("Unity-Version", &unity_version);

        let os_description = reader.read_string();
        cls_file_builder.add_header("Editor-Platform", &os_description);

        let build_target = reader.read_string();
        cls_file_builder.add_header("Build-Target", &build_target);

        let current_directory = reader.read_string();

        append_upm(&mut cls_file_builder, &current_directory);
        append_vpm(&mut cls_file_builder, &current_directory);

        let mut cls_file_builder = cls_file_builder.begin_body();

        let length: i32 = reader.read_i32();
        for _ in 0..length {
            let log_message = reader.read_string();
            let mode = reader.read_i32();
            cls_file_builder.add_header("Mode", &format!("{mode}")); // TODO: transfer to name
            cls_file_builder.add_header("Mode-Raw", &format!("{mode:08x}"));
            cls_file_builder.add_content("log-element", &log_message);
        }

        print!("{}", cls_file_builder.build());
    } else {
        eprintln!("version mismatch ({version})");
    }
}

struct TransferDataReader {
    reader: std::io::Cursor<Vec<u8>>,
}

impl TransferDataReader {
    fn new(data: Vec<u8>) -> Self {
        Self {
            reader: std::io::Cursor::new(data),
        }
    }

    fn read_i32(&mut self) -> i32 {
        self.reader.read_i32::<NativeEndian>().unwrap()
    }

    fn read_string(&mut self) -> String {
        let char_length = self.read_i32();
        let mut buffer = vec![0u16; char_length as usize];
        self.reader
            .read_u16_into::<NativeEndian>(buffer.as_mut_slice())
            .unwrap();
        String::from_utf16(&buffer).expect("bad utf16 message")
    }
}

fn append_upm(builder: &mut ClsHeadingBuilder, cwd: &str) {
    #[derive(Deserialize)]
    struct PackageLock {
        dependencies: std::collections::BTreeMap<String, UpmLockedDependency>,
    }
    #[derive(Deserialize)]
    struct UpmLockedDependency {
        version: Option<String>,
    }

    let package_lock = std::path::Path::new(cwd).join("Packages/packages-lock.json");
    let Ok(package_lock) = std::fs::read(&package_lock) else {
        return;
    };
    let Ok(package_lock) = serde_json::from_slice::<PackageLock>(&package_lock) else {
        return;
    };
    for (dependency, lock_info) in package_lock.dependencies {
        if let Some(version) = lock_info.version {
            builder.add_header("Upm-Dependency", &format!("{dependency}@{version}"));
        }
    }
}

fn append_vpm(builder: &mut ClsHeadingBuilder, cwd: &str) {
    #[derive(Deserialize)]
    struct PackageLock {
        locked: std::collections::BTreeMap<String, VpmLockedDependency>,
    }
    #[derive(Deserialize)]
    struct VpmLockedDependency {
        version: Option<String>,
    }

    let package_lock = std::path::Path::new(cwd).join("Packages/vpm-manifest.json");
    let Ok(package_lock) = std::fs::read(&package_lock) else {
        return;
    };
    let Ok(package_lock) = serde_json::from_slice::<PackageLock>(&package_lock) else {
        return;
    };
    for (dependency, lock_info) in package_lock.locked {
        if let Some(version) = lock_info.version {
            builder.add_header("Vpm-Dependency", &format!("{dependency}@{version}"));
        }
    }
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

enum MethodArg<'a> {
    Object(&'a SBValue),
    #[allow(dead_code)]
    Primitive(&'a SBValue),
    Literal(i64),
}

unsafe trait SBProcessExt {
    fn raw(&self) -> lldb::sys::SBProcessRef;

    fn read_memory(&self, addr: lldb_addr_t, buffer: &mut [u8]) -> Result<(), SBError> {
        unsafe {
            let error = SBError::default();
            lldb::sys::SBProcessReadMemory(
                self.raw(),
                addr,
                buffer.as_mut_ptr() as *mut _,
                buffer.len(),
                error.raw,
            );
            if error.is_success() {
                Ok(())
            } else {
                Err(error)
            }
        }
    }

    fn write_memory(&self, addr: lldb_addr_t, buffer: &[u8]) -> Result<(), SBError> {
        unsafe {
            let error = SBError::default();
            lldb::sys::SBProcessWriteMemory(
                self.raw(),
                addr,
                buffer.as_ptr() as *mut _,
                buffer.len(),
                error.raw,
            );
            if error.is_success() {
                Ok(())
            } else {
                Err(error)
            }
        }
    }

    fn byte_roder(&self) -> ByteOrder {
        unsafe { lldb::sys::SBProcessGetByteOrder(self.raw()) }
    }

    fn load_image(&self, file: &SBFileSpec) -> Result<u32, SBError> {
        unsafe {
            let error = SBError::default();
            let image_token = lldb::sys::SBProcessLoadImage(self.raw(), file.raw, error.raw);
            if error.is_failure() {
                Err(error)
            } else {
                Ok(image_token)
            }
        }
    }

    fn unload_image(&self, image_token: u32) -> Result<(), SBError> {
        unsafe {
            let error = lldb::sys::SBProcessUnloadImage(self.raw(), image_token);
            let error = SBError { raw: error };
            if error.is_failure() {
                Err(error)
            } else {
                Ok(())
            }
        }
    }

    fn target(&self) -> Option<SBTarget> {
        unsafe {
            let raw = lldb::sys::SBProcessGetTarget(self.raw());
            let target = SBTarget { raw };
            if target.is_valid() {
                Some(target)
            } else {
                None
            }
        }
    }
}

unsafe impl SBProcessExt for SBProcess {
    fn raw(&self) -> lldb::sys::SBProcessRef {
        self.raw
    }
}

unsafe trait SBFileSpecExt: Sized {
    fn from_raw(raw: lldb::sys::SBFileSpecRef) -> Self;

    fn from_path(path: &str) -> Self {
        let path_cstring = std::ffi::CString::new(path).unwrap();
        unsafe { Self::from_raw(lldb::sys::CreateSBFileSpec2(path_cstring.as_ptr())) }
    }
}

unsafe impl SBFileSpecExt for SBFileSpec {
    fn from_raw(raw: lldb::sys::SBFileSpecRef) -> Self {
        Self { raw }
    }
}

unsafe trait SBTargetExt {
    fn raw(&self) -> lldb::sys::SBTargetRef;

    fn byte_roder(&self) -> ByteOrder {
        unsafe { lldb::sys::SBTargetGetByteOrder(self.raw()) }
    }

    fn get_address_byte_size(&self) -> u32 {
        unsafe { lldb::sys::SBTargetGetAddressByteSize(self.raw()) }
    }
}

unsafe impl SBTargetExt for SBTarget {
    fn raw(&self) -> lldb::sys::SBTargetRef {
        self.raw
    }
}

unsafe trait SBDataExt {
    fn data_ref(&self) -> lldb::sys::SBDataRef;

    fn get_address(&self, offset: lldb_offset_t) -> Result<lldb_addr_t, SBError> {
        unsafe {
            let error = SBError::default();
            let result = lldb::sys::SBDataGetAddress(self.data_ref(), error.raw, offset);
            if error.is_success() {
                Ok(result)
            } else {
                Err(error)
            }
        }
    }

    fn read_raw(&self, offset: lldb_offset_t, buffer: &mut [u8]) -> Result<(), SBError> {
        unsafe {
            let error = SBError::default();
            lldb::sys::SBDataReadRawData(
                self.data_ref(),
                error.raw,
                offset,
                buffer.as_mut_ptr() as *mut _,
                buffer.len(),
            );
            lldb::sys::SBDataGetAddress(self.data_ref(), error.raw, offset);
            if error.is_success() {
                Ok(())
            } else {
                Err(error)
            }
        }
    }
}

unsafe impl SBDataExt for SBData {
    fn data_ref(&self) -> lldb::sys::SBDataRef {
        self.raw
    }
}

unsafe trait SBValueExt {
    fn data_ref(&self) -> lldb::sys::SBValueRef;

    fn get_signed(&self) -> Result<i64, SBError> {
        unsafe {
            let error = SBError::default();
            let result = lldb::sys::SBValueGetValueAsSigned(self.data_ref(), error.raw, 0);
            if error.is_success() {
                Ok(result)
            } else {
                Err(error)
            }
        }
    }
}

unsafe impl SBValueExt for SBValue {
    fn data_ref(&self) -> lldb::sys::SBValueRef {
        self.raw
    }
}

unsafe trait SBAddressExt {
    fn raw(&self) -> lldb::sys::SBAddressRef;

    fn get_offset(&self) -> lldb_addr_t {
        unsafe { lldb::sys::SBAddressGetOffset(self.raw()) }
    }

    fn get_section(&self) -> Option<SBSection> {
        unsafe {
            let section_ref = lldb::sys::SBAddressGetSection(self.raw());
            if section_ref.is_null() {
                None
            } else {
                Some(SBSection { raw: section_ref })
            }
        }
    }
}

unsafe impl SBAddressExt for SBAddress {
    fn raw(&self) -> lldb::sys::SBAddressRef {
        self.raw
    }
}

unsafe trait SBModuleSpecExt: Sized {
    fn from_raw(raw: lldb::sys::SBModuleSpecRef) -> Self;

    fn new() -> Self {
        Self::from_raw(unsafe { lldb::sys::CreateSBModuleSpec() })
    }
}
unsafe impl SBModuleSpecExt for SBModuleSpec {
    fn from_raw(raw: lldb::sys::SBModuleSpecRef) -> Self {
        Self { raw }
    }
}

unsafe trait SBModuleExt {
    fn raw(&self) -> lldb::sys::SBModuleRef;

    fn symbols(&self) -> ModuleSymbols {
        ModuleSymbols {
            module: self.raw(),
            _phantom: PhantomData,
        }
    }
}

unsafe impl SBModuleExt for SBModule {
    fn raw(&self) -> lldb::sys::SBModuleRef {
        self.raw
    }
}

struct ModuleSymbols<'a> {
    module: lldb::sys::SBModuleRef,
    _phantom: PhantomData<&'a SBModule>,
}

impl ModuleSymbols<'_> {
    pub fn len(&self) -> usize {
        unsafe { lldb::sys::SBModuleGetNumSymbols(self.module) }
    }

    pub fn get(&self, index: usize) -> Option<SBSymbol> {
        if index < self.len() {
            let symbol = unsafe { lldb::sys::SBModuleGetSymbolAtIndex(self.module, index) };
            Some(SBSymbol { raw: symbol })
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for ModuleSymbols<'a> {
    type Item = SBSymbol;
    type IntoIter = ModuleSymbolsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ModuleSymbolsIter {
            module: self,
            index: 0,
        }
    }
}

struct ModuleSymbolsIter<'a> {
    module: ModuleSymbols<'a>,
    index: usize,
}

impl Iterator for ModuleSymbolsIter<'_> {
    type Item = SBSymbol;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.module.len() {
            self.index += 1;
            self.module.get(self.index - 1)
        } else {
            None
        }
    }
}
