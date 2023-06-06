#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use libc::c_void;

#[repr(C)]
pub struct __CFDictionary(c_void);

pub type CFDictionaryRef = *const __CFDictionary;
pub type CFMutableDictionaryRef = *mut __CFDictionary;

pub type kern_return_t = i32;
pub type ipc_port_t = *mut c_void;
pub type mach_port_t = ipc_port_t;
pub type io_object_t = mach_port_t;
pub type io_connect_t = io_object_t;
pub type io_service_t = io_object_t;

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
        owningTask: libc::mach_port_t,
        r#type: u32,
        connect: *mut io_connect_t,
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

macro_rules! err_system {
    ( $err:literal ) => {
        (($err & 0x3f) << 26)
    };
}

macro_rules! err_sub {
    ( $err:literal ) => {
        (($err & 0xfff) << 14)
    };
}

pub const SYS_IOKIT: kern_return_t = err_system!(0x38);
pub const SUB_IOKIT_COMMON: kern_return_t = err_sub!(0);

macro_rules! iokit_common_err {
    ( $err:literal ) => {
        SYS_IOKIT | SUB_IOKIT_COMMON | $err
    };
}

pub const KERN_SUCCESS: kern_return_t = 0;
pub const kIOReturnSuccess: kern_return_t = KERN_SUCCESS;
pub const kIOReturnNotPrivileged: kern_return_t = iokit_common_err!(0x2c1);

pub const MACH_PORT_NULL: mach_port_t = 0 as mach_port_t;
pub const kIOMasterPortDefault: mach_port_t = MACH_PORT_NULL;

pub const HW_PACKAGES: i32 = 125;
pub const HW_PHYSICALCPU: i32 = 101;
