use bytemuck::{AnyBitPattern, NoUninit};
use lldb::{lldb_addr_t, lldb_offset_t, lldb_pid_t, ByteOrder, FunctionNameType, Permissions, SBAddress, SBAttachInfo, SBData, SBDebugger, SBError, SBExpressionOptions, SBFileSpec, SBFrame, SBListener, SBModule, SBModuleSpec, SBProcess, SBSection, SBSymbol, SBTarget, SBValue, SymbolType};
use std::env::args;
use std::ffi::CStr;
use std::io::Write;
use std::marker::PhantomData;
use std::process::{exit, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};
use byteorder::{NativeEndian, ReadBytesExt, WriteBytesExt};

fn main() {
    let mut args = args();

    let unity_pid = args
        .nth(1)
        .expect("please specify pid")
        .parse::<lldb_pid_t>()
        .expect("Failed to parse unity pid");

    SBDebugger::initialize();

    #[cfg(all(not(unix), feature = "external_debug_server"))]
    compile_error!("external_debug_server feature is only for unix platform");

    #[cfg(all(unix, not(feature = "external_debug_server")))]
    let _named_temp = {
        use std::os::unix::fs::PermissionsExt;

        let mut named_temp = tempfile::Builder::new()
            .prefix("cls-lldb-debugserver")
            .suffix(".exe")
            .tempfile()
            .expect("failed to create temporary file");

        named_temp.as_file_mut().set_permissions(std::fs::Permissions::from_mode(0o755)).expect("failed to set permissions");
        named_temp.write_all(include_bytes!(env!("LLDB_BUNDLE_DEBUGSERVER_PATH"))).expect("creating debugserver failed");

        unsafe {
            std::env::set_var("LLDB_DEBUGSERVER_PATH", named_temp.path());
        }

        named_temp
    };

    #[cfg(all(unix, feature = "external_debug_server"))]
    {
        let debugserver = env!("LLDB_REFERENCE_DEBUGSERVER_PATH");
        if let Some(relative_path) = debugserver.strip_prefix("@executable/") {
            let mut executable_path = std::env::current_exe().expect("Failed to get current executable path");
            executable_path.push(relative_path);
            std::env::set_var("LLDB_DEBUGSERVER_PATH", executable_path);
        } else {
            std::env::set_var("LLDB_DEBUGSERVER_PATH", debugserver);
        }
    }

    let attach_lib_dylib = {
        #[cfg(target_os = "windows")]
        let suffix = ".exe";
        #[cfg(target_os = "macos")]
        let suffix = ".dylib";
        #[cfg(target_os = "linux")]
        let suffix = ".so";

        let mut named_temp = tempfile::Builder::new()
            .prefix("cls_attach_lib")
            .suffix(suffix)
            .tempfile()
            .expect("failed to create temporary file");

        named_temp.write_all(include_bytes!(env!("CLS_ATTACH_LIB_PATH"))).expect("creating cls attach library failed");

        named_temp
    };
    let attach_lib_dylib_path = attach_lib_dylib.into_temp_path();

    let debugger = SBDebugger::create(false);
    debugger.set_asynchronous(false);

    // reading symbol table took some time, we want to skip
    let target = debugger.create_target("", None, None, false).unwrap();

    let attach = Instant::now();
    let attach_info = SBAttachInfo::new_with_pid(unity_pid);

    let process = target.attach(attach_info).unwrap();
    println!("Attaching process took {:?}, running?: {}", attach.elapsed(), process.is_running());

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

    struct LoadImageResult<F> {
        saver_save: lldb_addr_t,
        location: lldb_addr_t,
        unload: F,
    }

    #[cfg(windows)]
    fn load_image(process: &SBProcess, load_path: &std::path::Path) -> Result<LoadImageResult<impl FnOnce()>, SBError> {
        // on windows, we can find_module for modules we just loaded, so we use 
        let target = process.target().unwrap();

        let path = load_path.to_str().unwrap();
        let dylib = SBFileSpec::from_path(path);
        let image_token = process.load_image(&dylib).expect("loading image");

        // not working on posix (at least macos)
        let dylib = target.find_module(&dylib).expect("loaded dylib not found");

        let saver_save = dylib.find_functions("CONSOLE_LOG_SAVER_SAVE", FunctionNameType::AUTO.bits())
            .iter()
            .nth(0)
            .unwrap()
            .symbol()
            .start_address()
            .unwrap()
            .load_address(&target);
        let location = dylib.find_symbols("CONSOLE_LOG_SAVER_SAVED_LOCATION", SymbolType::Data)
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
            location,
            unload: move || {
                process.unload_image(image_token).expect("unloading image");
            }
        })
    }

    #[cfg(unix)]
    fn load_image(process: &SBProcess, load_path: &std::path::Path) -> Result<LoadImageResult<impl FnOnce()>, SBError> {
        use std::os::unix::ffi::OsStrExt;
        // on unix, we cannot find_module for modules we just loaded,
        // so we use directly calling dlopen 
        let frame = process.selected_thread().frames().nth(0).unwrap();

        /*
        C-like expression:
        struct InOut {
          // input data
          const char *load_path, // 0
          // constants. For easier writing code, we pass constants with inoput struct
          const char *saver_save_name, // 1
          const char *location_name, // 2
          // error data.
          const char *error, // 3
          size_t error_len, // 4
          // success data
          void *handle, // 5
          void *saver_save, // 6
          void *location, // 7
        }

        #define RTLD_LAZY 1
        InOut *input = <pointer>;

        input.handle = void *handle = dlopen(input->load_path, RTLD_LAZY);
        if (handle == NULL) goto error;

        const char *saver_save_name = input->saver_save_name;
        input.saver_save = void *saver_save = dlsym(handle, saver_save_name);
        if (saver_save == NULL) goto error;

        const char *location_name = input->location_name;
        input.location   = void *location   = dlsym(handle, location_name);
        if (location == NULL) goto error;

        goto return;
       error:
        input.error = void *error = dlerror();
        if (error != null) input.error_len = strlen(error);
        if (handle != null) dlclose(handle);
        goto return;
       return:
        return;
         */

        let load_path_idx: usize = 0;
        let saver_save_name_idx: usize = 1;
        let location_name_idx: usize = 2;
        let error_idx: usize = 3;
        let error_len_idx: usize = 4;
        let handle_idx: usize = 5;
        let saver_save_idx: usize = 6;
        let location_idx: usize = 7;
        const INOUT_ELEMENT_COUNT: usize = 8;

        let saver_save_name = "CONSOLE_LOG_SAVER_SAVE";
        let location_name = "CONSOLE_LOG_SAVER_SAVED_LOCATION";

        const STRUCT_DATA_SIZE: usize = size_of::<usize>() * INOUT_ELEMENT_COUNT;

        let mut buffer = Vec::with_capacity(STRUCT_DATA_SIZE
            + (load_path.as_os_str().len() + 1)
            + (saver_save_name.len() + 1)
            + (location_name.len() + 1));
        buffer.extend_from_slice(&[0u8; STRUCT_DATA_SIZE]);
        let mut buffer_writer = std::io::Cursor::new(&mut buffer);

        fn write_get_ptr(writer: &mut std::io::Cursor<&mut Vec<u8>>, data: &[u8]) -> u64 {
            let offset = writer.position();
            writer.write_all(data).unwrap();
            writer.write_all(b"\0").unwrap();
            offset
        }

        buffer_writer.set_position(STRUCT_DATA_SIZE as u64);
        let load_path_offset = write_get_ptr(&mut buffer_writer, &load_path.as_os_str().as_bytes());
        let saver_save_name_offset = write_get_ptr(&mut buffer_writer, &saver_save_name.as_bytes());
        let location_name_offset = write_get_ptr(&mut buffer_writer, &location_name.as_bytes());

        let buffer_location = process.allocate_memory(buffer.len(), Permissions::READABLE | Permissions::WRITABLE).expect("allocating memory");

        fn set_usize(buffer: &mut Vec<u8>, index: usize, value: usize) {
            let offset = index * size_of::<usize>();
            buffer[offset..][..size_of::<usize>()].copy_from_slice(&value.to_ne_bytes());
        }

        set_usize(&mut buffer, load_path_idx, (buffer_location + load_path_offset) as usize);
        set_usize(&mut buffer, saver_save_name_idx, (buffer_location + saver_save_name_offset) as usize);
        set_usize(&mut buffer, location_name_idx, (buffer_location + location_name_offset) as usize);

        process.write_memory(buffer_location, &buffer).expect("writing memory");
        drop(buffer);

        let error_block = 3;
        let ok_block = 7;

        let expression = format!(r#"
#!mini-llvm-expr 8
define_struct InOut ptr ptr ptr ptr iptr ptr ptr ptr
; functions
; declare ptr @dlopen(ptr, i32)
define_function_type ptr dlopen ptr i32
declare_function dlopen dlopen

; declare ptr @dlerror()
define_function_type ptr dlerror
declare_function dlerror dlerror

; declare ptr @dlsym(ptr, ptr)
define_function_type ptr dlsym ptr ptr
declare_function dlsym dlsym

; declare iptr @strlen(ptr)
define_function_type iptr strlen ptr
declare_function strlen strlen

; declare i32 @dlclose(ptr)
define_function_type i32 dlclose ptr
declare_function dlclose dlclose
; constants
const RTLD_LAZY i32 1
const iptr_0 iptr 0
const load_path_idx i32 {load_path_idx}
const saver_save_name_idx i32 {saver_save_name_idx}
const location_name_idx i32 {location_name_idx}
const error_idx i32 {error_idx}
const error_len_idx i32 {error_len_idx}
const handle_idx i32 {handle_idx}
const saver_save_idx i32 {saver_save_idx}
const location_idx i32 {location_idx}
const input ptr {buffer_location}

const iptr_1 iptr 1
const iptr_2 iptr 2
const iptr_3 iptr 3

; main expression
begin_block 0
  load load_path ptr input
  call handle dlopen dlopen load_path RTLD_LAZY
  getelementptr handle_ptr InOut input iptr_0 handle_idx
  store handle handle_ptr
  icmp handle_is_null_0 eq handle null
  cond_br handle_is_null_0 {error_block} 1
  
begin_block 1
  getelementptr saver_save_name_ptr InOut input iptr_0 saver_save_name_idx
  load saver_save_name ptr saver_save_name_ptr
  call saver_save dlsym dlsym handle saver_save_name
  getelementptr saver_save_ptr InOut input iptr_0 saver_save_idx
  store saver_save saver_save_ptr
  icmp saver_save_is_null eq saver_save null
  cond_br saver_save_is_null {error_block} 2
  
begin_block 2
  getelementptr location_name_ptr InOut input iptr_0 location_name_idx
  load location_name ptr location_name_ptr
  call location dlsym dlsym handle location_name
  getelementptr location_ptr InOut input iptr_0 location_idx
  store location location_ptr
  icmp location_is_null eq location null
  cond_br location_is_null {error_block} {ok_block}

begin_block 3 # error_block
  call error dlerror dlerror
  getelementptr error_ptr InOut input iptr_0 error_idx
  store error error_ptr
  icmp error_is_null eq error null
  cond_br error_is_null 5 4

begin_block 4
  call error_len strlen strlen error
  getelementptr error_len_ptr InOut input iptr_0 error_len_idx
  store error_len error_len_ptr
  br 5

begin_block 5
  icmp handle_is_null_2 eq handle null
  cond_br handle_is_null_2 7 6
begin_block 6
  call _ dlclose dlclose handle
  br 7

begin_block 7 # ok_block
  ret_void
"#);

        let options = SBExpressionOptions::new();
        let result = frame.evaluate_expression(&expression, &options);

        let mut read_buffer = [0usize; INOUT_ELEMENT_COUNT];
        process.read_memory(buffer_location, bytemuck::cast_slice_mut(&mut read_buffer)).expect("reading memory");

        // 0x1001 is kNoResult, which is not an error
        // https://github.com/llvm/llvm-project/blob/d6e65a66095cc3c93ea78669bc41d0885780e8ea/lldb/include/lldb/Expression/UserExpression.h#L274
        if result
            .error()
            .map(|x| x.is_failure() && x.error() != 0x1001)
            .unwrap_or(false)
        {
            panic!("{}", result.error().unwrap());
        }

        let handle = read_buffer[handle_idx];
        let saver_save = read_buffer[saver_save_idx] as lldb_addr_t;
        let location = read_buffer[location_idx] as lldb_addr_t;
        let error = read_buffer[error_idx];

        if error != 0 || handle == 0 || saver_save == 0 || location == 0 {
            if error != 0 {
                // there is error message from dlerror
                let error_len = read_buffer[error_len_idx];
                let mut error_buffer = vec![0u8; error_len + 1];
                process.read_memory(error as lldb_addr_t, &mut error_buffer).expect("reading error message");
                let error_message = CStr::from_bytes_with_nul(&error_buffer).expect("bad error msssage");
                let message = error_message.to_str().expect("bad utf8 message");
                panic!("dlopen or dlsym failed with error: {message}")
            } else {
                panic!("dlopen or dlsym failed with unknown error")
            }
        }

        let unload = move || {
            let options = SBExpressionOptions::new();
            frame.evaluate_expression(&format!(r#"
#!mini-llvm-expr 1
define_function_type i32 dlclose ptr
declare_function dlclose dlclose

const target_ptr ptr {handle}
call ret dlclose dlclose target_ptr
ret_void"#), &options);
        };

        Ok(LoadImageResult {
            saver_save,
            location,
            unload,
        })
    }

    {
        let load_image = load_image(&process, attach_lib_dylib_path.as_ref()).expect("load_image");

        let saver_save = load_image.saver_save;
        let location = load_image.location;

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

        (load_image.unload)();
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
