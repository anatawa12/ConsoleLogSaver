#![allow(dead_code)]

use lldb::{lldb_addr_t, Permissions, SBFrame, SBProcess};
use std::convert::Infallible;
use std::ffi::CStr;
use std::io::Write;
use tempfile::TempPath;

pub struct LoadImageResult {
    saver_save: lldb_addr_t,
    free_mem: lldb_addr_t,
    location: lldb_addr_t,
    handle: usize,
    frame: SBFrame,
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
        let handle = self.handle;
        super::eval_expr(
            &self.frame,
            &format!(
                r#"
#!mini-llvm-expr 1
define_function_type i32 dlclose ptr
declare_function dlclose dlclose

const target_ptr ptr {handle}
call ret dlclose dlclose target_ptr
ret_void"#
            ),
        )
        .ok(); // ignore error
    }
}

#[cfg(unix)]
pub fn load_image(
    process: &SBProcess,
    load_path: &std::path::Path,
) -> Result<LoadImageResult, Infallible> {
    use std::os::unix::ffi::OsStrExt;
    // on unix, we cannot find_module for modules we just loaded,
    // so we use directly calling dlopen
    let frame = process.selected_thread().frames().nth(0).unwrap();

    /*
     C-like expression:
     struct InOut {
       // input data
       const char *load_path,
       // constants. For easier writing code, we pass constants with inoput struct
       const char *saver_save_name,
       const char *free_mem_name,
       const char *location_name,
       // error data.
       const char *error,
       size_t error_len,
       // success data
       void *handle,
       void *saver_save,
       void *free_mem_save,
       void *location,
     }

     #define RTLD_LAZY 1
     InOut *input = <pointer>;

     input.handle = void *handle = dlopen(input->load_path, RTLD_LAZY);
     if (handle == NULL) goto error;

     const char *saver_save_name = input->saver_save_name;
     input.saver_save = void *saver_save = dlsym(handle, saver_save_name);
     if (saver_save == NULL) goto error;

     const char *free_mem_name = input->free_mem_name;
     input.free_mem = void *free_mem = dlsym(handle, free_mem_name);
     if (free_mem == NULL) goto error;

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
    let free_mem_name_idx: usize = 2;
    let location_name_idx: usize = 3;
    let error_idx: usize = 4;
    let error_len_idx: usize = 5;
    let handle_idx: usize = 6;
    let saver_save_idx: usize = 7;
    let free_mem_idx: usize = 8;
    let location_idx: usize = 9;
    const INOUT_ELEMENT_COUNT: usize = 10;

    let saver_save_name = "CONSOLE_LOG_SAVER_SAVE";
    let free_mem_name = "CONSOLE_LOG_SAVER_FREE_MEM";
    let location_name = "CONSOLE_LOG_SAVER_SAVED_LOCATION";

    const STRUCT_DATA_SIZE: usize = size_of::<usize>() * INOUT_ELEMENT_COUNT;

    let mut buffer = Vec::with_capacity(
        STRUCT_DATA_SIZE
            + (load_path.as_os_str().len() + 1)
            + (saver_save_name.len() + 1)
            + (free_mem_name.len() + 1)
            + (location_name.len() + 1),
    );
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
    let free_mem_name_offset = write_get_ptr(&mut buffer_writer, &free_mem_name.as_bytes());
    let location_name_offset = write_get_ptr(&mut buffer_writer, &location_name.as_bytes());

    let buffer_location = process
        .allocate_memory(buffer.len(), Permissions::READABLE | Permissions::WRITABLE)
        .expect("allocating memory");

    fn set_usize(buffer: &mut Vec<u8>, index: usize, value: usize) {
        let offset = index * size_of::<usize>();
        buffer[offset..][..size_of::<usize>()].copy_from_slice(&value.to_ne_bytes());
    }

    set_usize(
        &mut buffer,
        load_path_idx,
        (buffer_location + load_path_offset) as usize,
    );
    set_usize(
        &mut buffer,
        saver_save_name_idx,
        (buffer_location + saver_save_name_offset) as usize,
    );
    set_usize(
        &mut buffer,
        free_mem_name_idx,
        (buffer_location + free_mem_name_offset) as usize,
    );
    set_usize(
        &mut buffer,
        location_name_idx,
        (buffer_location + location_name_offset) as usize,
    );

    process
        .write_memory(buffer_location, &buffer)
        .expect("writing memory");
    drop(buffer);

    let error_block = 4;
    let ok_block = 8;

    let expression = format!(
        r#"
#!mini-llvm-expr 9
define_struct InOut ptr ptr ptr ptr ptr iptr ptr ptr ptr ptr
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
const free_mem_name_idx i32 {free_mem_name_idx}
const location_name_idx i32 {location_name_idx}
const error_idx i32 {error_idx}
const error_len_idx i32 {error_len_idx}
const handle_idx i32 {handle_idx}
const saver_save_idx i32 {saver_save_idx}
const free_mem_idx i32 {free_mem_idx}
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
  getelementptr free_mem_name_ptr InOut input iptr_0 free_mem_name_idx
  load free_mem_name ptr free_mem_name_ptr
  call free_mem dlsym dlsym handle free_mem_name
  getelementptr free_mem_ptr InOut input iptr_0 free_mem_idx
  store free_mem free_mem_ptr
  icmp free_mem_is_null eq free_mem null
  cond_br free_mem_is_null {error_block} 3

begin_block 3
  getelementptr location_name_ptr InOut input iptr_0 location_name_idx
  load location_name ptr location_name_ptr
  call location dlsym dlsym handle location_name
  getelementptr location_ptr InOut input iptr_0 location_idx
  store location location_ptr
  icmp location_is_null eq location null
  cond_br location_is_null {error_block} {ok_block}

begin_block 4 # error_block
  call error dlerror dlerror
  getelementptr error_ptr InOut input iptr_0 error_idx
  store error error_ptr
  icmp error_is_null eq error null
  cond_br error_is_null 6 5

begin_block 5
  call error_len strlen strlen error
  getelementptr error_len_ptr InOut input iptr_0 error_len_idx
  store error_len error_len_ptr
  br 6

begin_block 6
  icmp handle_is_null_2 eq handle null
  cond_br handle_is_null_2 8 7
begin_block 7
  call _ dlclose dlclose handle
  br 8

begin_block 8 # ok_block
  ret_void
"#
    );

    super::eval_expr(&frame, &expression).expect("loading memory");

    let mut read_buffer = [0usize; INOUT_ELEMENT_COUNT];
    process
        .read_memory(buffer_location, bytemuck::cast_slice_mut(&mut read_buffer))
        .expect("reading memory");

    unsafe { process.deallocate_memory(buffer_location) }.expect("deallocating memory");

    let handle = read_buffer[handle_idx];
    let saver_save = read_buffer[saver_save_idx] as lldb_addr_t;
    let free_mem = read_buffer[free_mem_idx] as lldb_addr_t;
    let location = read_buffer[location_idx] as lldb_addr_t;
    let error = read_buffer[error_idx];

    if error != 0 || handle == 0 || saver_save == 0 || location == 0 {
        if error != 0 {
            // there is error message from dlerror
            let error_len = read_buffer[error_len_idx];
            let mut error_buffer = vec![0u8; error_len + 1];
            process
                .read_memory(error as lldb_addr_t, &mut error_buffer)
                .expect("reading error message");
            let error_message =
                CStr::from_bytes_with_nul(&error_buffer).expect("bad error msssage");
            let message = error_message.to_str().expect("bad utf8 message");
            panic!("dlopen or dlsym failed with error: {message}")
        } else {
            panic!("dlopen or dlsym failed with unknown error")
        }
    }

    Ok(LoadImageResult {
        saver_save,
        free_mem,
        location,
        handle,
        frame,
    })
}

#[cfg(all(unix, not(feature = "external_debug_server")))]
pub fn prepare_debug_server() -> Option<TempPath> {
    use std::os::unix::fs::PermissionsExt;

    let mut debugserver = tempfile::Builder::new()
        .prefix("cls-lldb-debugserver")
        .suffix(".exe")
        .tempfile()
        .expect("failed to create temporary file");

    debugserver
        .as_file_mut()
        .set_permissions(std::fs::Permissions::from_mode(0o755))
        .expect("failed to set permissions");
    debugserver
        .write_all(include_bytes!(env!("LLDB_BUNDLE_DEBUGSERVER_PATH")))
        .expect("creating debugserver failed");

    unsafe {
        std::env::set_var("LLDB_DEBUGSERVER_PATH", debugserver.path());
    }

    Some(debugserver.into_temp_path())
}

#[cfg(all(unix, feature = "external_debug_server"))]
pub fn prepare_debug_server() -> Option<TempPath> {
    let debugserver = env!("LLDB_REFERENCE_DEBUGSERVER_PATH");

    if let Some(relative_path) = debugserver.strip_prefix("@executable/") {
        let mut executable_path =
            std::env::current_exe().expect("Failed to get current executable path");
        executable_path.push(relative_path);
        unsafe {
            std::env::set_var("LLDB_DEBUGSERVER_PATH", executable_path);
        }
    } else {
        unsafe {
            std::env::set_var("LLDB_DEBUGSERVER_PATH", debugserver);
        }
    }

    None
}
