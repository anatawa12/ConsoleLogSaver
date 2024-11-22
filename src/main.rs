use bytemuck::{AnyBitPattern, NoUninit};
use lldb::{lldb_addr_t, lldb_offset_t, lldb_pid_t, ByteOrder, FunctionNameType, SBAddress, SBAttachInfo, SBData, SBDebugger, SBError, SBExpressionOptions, SBFileSpec, SBFrame, SBListener, SBModule, SBModuleSpec, SBProcess, SBSection, SBSymbol, SBTarget, SBValue, SymbolType};
use std::env::args;
use std::io::Write;
use std::marker::PhantomData;
use std::os::unix::fs::PermissionsExt;
use std::process::{exit, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};
use byteorder::{NativeEndian, ReadBytesExt};

fn main() {
    let mut args = args();

    let unity_pid = args
        .nth(1)
        .expect("please specify pid")
        .parse::<lldb_pid_t>()
        .expect("Failed to parse unity pid");

    SBDebugger::initialize();

    let mut named_temp = tempfile::Builder::new()
        .prefix("cls-lldb-debugserver")
        .suffix(".exe")
        .tempfile()
        .expect("failed to create temporary file");

    named_temp.as_file_mut().set_permissions(std::fs::Permissions::from_mode(0o755)).expect("failed to set permissions");
    named_temp.write_all(include_bytes!("/Users/anatawa12/CLionProjects/llvm-project/build/bin/debugserver")).expect("creating debugserver failed");

    unsafe {
        std::env::set_var("LLDB_DEBUGSERVER_PATH", named_temp.path());
    }

    let debugger = SBDebugger::create(false);
    debugger.set_asynchronous(false);

    // reading symbol table took some time, we want to skip
    let target = debugger.create_target("", None, None, false).unwrap();

    let attach = Instant::now();
    let attach_info = SBAttachInfo::new_with_pid(unity_pid);

    let process = target.attach(attach_info).unwrap();
    println!("Attaching process took {:?}, running?: {}", attach.elapsed(), process.is_running());
    
    println!("removing temp server");
    drop(named_temp);
    //sleep(Duration::from_secs(30));

    let before_break = Instant::now();

    // I don't know wht but target.find_functions("SceneTracker::Update") don't work on windows
    // so we use different method
    let mut update = None;
    'modules: for module in target.modules() {
        if !module.filespec().filename().contains("Unity") {
            continue;
        }
        for symbol in module.symbols() {
            //println!("Processing symbol {:?}", symbol);
            if symbol.name().contains("SceneTracker::Update(") {
                update = Some(symbol.start_address().expect("no start address for SceneTracker::Update"));
                break 'modules;
            }
        }
    }

    let update = update.expect("SceneTracker::Update symbol not found");

    let breakpoint = target.breakpoint_create_by_sbaddress(update);
    breakpoint.set_enabled(true);
    breakpoint.set_oneshot(true);

    process.continue_execution().unwrap();

    println!("continue to breakpoint took {:?}", before_break.elapsed());

    if target.byte_roder() != current_byte_order() {
        eprintln!(
            "byte order mismatch (target={target:?},current={current:?})",
            target = target.byte_roder(),
            current = current_byte_order(),
        );
        exit(1);
    }

    if target.get_address_byte_size() as usize != size_of::<usize>() {
        eprintln!("pointer size mismatch");
        exit(1);
    }

    let thread = process.selected_thread();
    let frame = thread.frames().nth(0).unwrap();

    let ctx = LLDBContext::new(&target, &process, &frame);

    {
        let path = "/Users/anatawa12/RustroverProjects/console-log-saver/target/debug/libcls_attach_lib.dylib";
        let dylib = SBFileSpec::from_path(path);
        let image_token = process.load_image(&dylib).expect("loading image");

        let saver_save = ctx.get_function_addr("CONSOLE_LOG_SAVER_SAVE");
        let location = ctx.get_addr("CONSOLE_LOG_SAVER_SAVED_LOCATION");
        println!("saver save address: {}", saver_save);
        println!("saver location address: {}", location);

        ctx.eval(&format!(r##"
        #!mini-llvm-expr 1
        const target_ptr ptr {saver_save}
        define_function_type void void_no_arg
        call _ void_no_arg target_ptr
        ret_void
        "##));

        let mut pointer = 0usize;
        process.read_memory(location, bytemuck::cast_slice_mut(std::slice::from_mut(&mut pointer))).expect("reading pointer");
        let pointer = pointer as lldb_addr_t;

        let mut data_size = 0u64;
        process.read_memory(pointer, bytemuck::cast_slice_mut(std::slice::from_mut(&mut data_size))).expect("reading size memory");

        if data_size >= usize::MAX as u64 {
            panic!("size overflow");
        }

        // similar to vec![0u8; data_size as usize] but aligned to align_of::<u64>()
        let mut buffer = unsafe { 
            let size = data_size as usize;
            let align = align_of::<u64>();
            let layout = std::alloc::Layout::from_size_align_unchecked(size, align);
            let mem = std::alloc::alloc(layout);
            if mem.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            // make sure initialized with 0 before creating vector
            mem.write_bytes(0, size);

            Vec::from_raw_parts(mem, size, size)
        };

        let mut buffer = vec![0u8; data_size as usize];
        process.read_memory(pointer + 8, &mut buffer).expect("reading data memory");

        let mut reader = std::io::Cursor::new(&buffer);
        let version: i32 = reader.read_i32::<NativeEndian>().unwrap();
        if version == 1 {
            let length: i32 = reader.read_i32::<NativeEndian>().unwrap();
            for i in 0..length {
                let char_length: i32 = reader.read_i32::<NativeEndian>().unwrap();
                let mut buffer = vec![0u16; char_length as usize];
                reader.read_u16_into::<NativeEndian>(buffer.as_mut_slice()).unwrap();
                let log_message = String::from_utf16(&buffer).unwrap();
                println!("log message: of {i}\n{log_message}");
            }
        } else {
            println!("version mismatch ({version})");
        }

        process.unload_image(image_token).expect("unloading image");
    }

    // I don't know why but detaching with synchronous and no resume 
    // would freeze target process on detach after loading image.
    debugger.set_asynchronous(true);
    process.continue_execution().unwrap();

    process.detach().unwrap();

    SBDebugger::terminate();
}

struct LLDBContext<'a> {
    target: &'a SBTarget,
    process: &'a SBProcess,
    frame: &'a SBFrame,
}

impl<'a> LLDBContext<'a> {
    fn new(target: &'a SBTarget, process: &'a SBProcess, frame: &'a SBFrame) -> Self {
        Self {
            target,
            process,
            frame,
        }
    }
}

impl LLDBContext<'_> {
    fn eval(&self, expr: &str) -> SBValue {
        let options = SBExpressionOptions::new();
        let result = self.frame.evaluate_expression(expr, &options);
        // 0x1001 is kNoResult, which is not an error
        // https://github.com/llvm/llvm-project/blob/d6e65a66095cc3c93ea78669bc41d0885780e8ea/lldb/include/lldb/Expression/UserExpression.h#L274
        if result
            .error()
            .map(|x| x.is_failure() && x.error() != 0x1001)
            .unwrap_or(false)
        {
            panic!("{}", result.error().unwrap());
        }
        result
    }

    fn get_function_addr(&self, name: &str) -> u64 {
        self.target
            .find_functions(name, FunctionNameType::AUTO.bits())
            .iter()
            .nth(0)
            .unwrap()
            .symbol()
            .start_address()
            .unwrap()
            .load_address(self.target)
    }

    fn get_addr(&self, name: &str) -> u64 {
        self.target
            .find_symbols(name, SymbolType::Data)
            .iter()
            .nth(0)
            .unwrap()
            .symbol()
            .start_address()
            .unwrap()
            .load_address(self.target)
    }

    fn read_array<T: NoUninit + AnyBitPattern + Default>(
        &self,
        length: usize,
        ptr: lldb_addr_t,
    ) -> Vec<T> {
        let mut buffer = vec![T::default(); length];
        self.process
            .read_memory(ptr, bytemuck::cast_slice_mut(&mut buffer))
            .unwrap();
        buffer
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
}

unsafe impl SBProcessExt for SBProcess {
    fn raw(&self) -> lldb::sys::SBProcessRef {
        self.raw
    }
}

unsafe trait SBFileSpecExt : Sized {
    fn from_raw(raw: lldb::sys::SBFileSpecRef) -> Self;

    fn from_path(path: &str) -> Self {
        let path_cstring = std::ffi::CString::new(path).unwrap();
        unsafe {
            Self::from_raw(lldb::sys::CreateSBFileSpec2(path_cstring.as_ptr()))
        }
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

unsafe trait SBModuleSpecExt : Sized {
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
