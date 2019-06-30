#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use std::os::raw::c_void;

#[repr(C)]
pub struct __CFDictionary(c_void);

pub type CFDictionaryRef = *const __CFDictionary;
pub type CFMutableDictionaryRef = *mut __CFDictionary;

pub type kern_return_t = i32;
pub type ipc_port_t = *mut c_void;
pub type mach_port_t = ipc_port_t;
pub type io_object_t = mach_port_t;
pub type io_connect_t = io_object_t;
pub type task_t = *mut c_void;
pub type task_port_t = task_t;
pub type io_service_t = io_object_t;

extern "C" {
    pub fn mach_task_self() -> mach_port_t;
}

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    pub fn IOServiceMatching(name: *const u8) -> CFMutableDictionaryRef;
    pub fn IOServiceGetMatchingService(
        masterPort: mach_port_t,
        matching: CFDictionaryRef,
    ) -> io_service_t;
    pub fn IOObjectRelease(object: io_object_t) -> kern_return_t;
    pub fn IOServiceOpen(
        service: io_service_t,
        owningTask: task_port_t,
        r#type: u32,
        connect: *const io_connect_t,
    ) -> kern_return_t;
    pub fn IOServiceClose(connect: io_connect_t) -> kern_return_t;
    pub fn IOConnectCallStructMethod(
        connection: mach_port_t,
        selector: u32,
        inputStruct: *const c_void,
        inputStructCnt: usize,
        outputStruct: *mut c_void,
        outputStructCnt: *mut usize,
    ) -> kern_return_t;
}
