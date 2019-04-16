#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

#[cfg(not(target_os = "macos"))]
compile_error!("SMC only works on macOS");

extern crate ctor;
extern crate core_foundation;
extern crate libc;

use ctor::*;
use core_foundation::dictionary::{ CFMutableDictionaryRef, CFDictionaryRef };
use std::os::raw::{ c_void, c_char, c_uchar, c_int, c_uint };
use std::sync::atomic::{ AtomicUsize, Ordering };
use std::ffi::{ CString, CStr };

macro_rules! c_str {
    ($lit:ident) => {
        c_str!(stringify!($lit))
    };
    ($lit:expr) => {
        concat!($lit, "\0").as_ptr() as *const ::std::os::raw::c_char
    }
}

const KERNEL_INDEX_SMC: u32 = 2;

const SMC_CMD_READ_BYTES: c_char = 5;
const SMC_CMD_READ_KEYINFO: c_char = 9;

const DATATYPE_FPE2: *const c_char = c_str!(fpe2);
const DATATYPE_SP78: *const c_char = c_str!(sp78);

const SMC_KEY_CPU_TEMP: *const c_char = c_str!(TC0P);
const SMC_KEY_GPU_TEMP: *const c_char = c_str!(TG0P);

type kern_return_t = c_int;
type ipc_port_t = *const c_void;
type mach_port_t = ipc_port_t;
type io_object_t = mach_port_t;
type io_connect_t = io_object_t;
type io_iterator_t = io_object_t;
type task_t = *const c_void;
type task_port_t = task_t;
type io_service_t = io_object_t;

#[repr(C)]
struct SMCKeyData_vers_t {
    major: c_char,
    minor: c_char,
    build: c_char,
    reserved: [c_char; 1],
    release: u16,
}

#[repr(C)]
struct SMCKeyData_pLimitData_t {
    version: u16,
    length: u16,
    cpuPLimit: u32,
    gpuPLimit: u32,
    memPLimit: u32,
}

#[repr(C)]
struct SMCKeyData_keyInfo_t {
    dataSize: u32,
    dataType: u32,
    dataAttributes: c_char,
}

type SMCBytes_t = [c_char; 32];

#[repr(C)]
struct SMCKeyData_t {
    key: u32,
    vers: SMCKeyData_vers_t,
    pLimitData: SMCKeyData_pLimitData_t,
    keyInfo: SMCKeyData_keyInfo_t,
    result: c_char,
    status: c_char,
    data8: c_char,
    data32: u32,
    bytes: SMCBytes_t,
}

type UInt32Char_t = [c_char; 5];

#[repr(C)]
struct SMCVal_t {
    key: UInt32Char_t,
    dataSize: u32,
    dataType: UInt32Char_t,
    bytes: SMCBytes_t,
}

const KERN_SUCCESS: kern_return_t = 0;
const kIOReturnSuccess: kern_return_t = KERN_SUCCESS;

const MACH_PORT_NULL: mach_port_t = 0 as mach_port_t;
const kIOMasterPortDefault: mach_port_t = MACH_PORT_NULL;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn mach_task_self() -> mach_port_t;
    fn IOServiceMatching(name: *const c_char) -> CFMutableDictionaryRef;
    fn IOServiceGetMatchingServices(masterPort: mach_port_t, matching: CFDictionaryRef, existing: *const io_iterator_t) -> kern_return_t;
    fn IOIteratorNext(iterator: io_iterator_t) -> io_object_t;
    fn IOObjectRelease(object: io_object_t) -> kern_return_t;
    fn IOServiceOpen(service: io_service_t, owningTask: task_port_t, r#type: u32, connect: *const io_connect_t) -> kern_return_t;
    fn IOServiceClose(connect: io_connect_t) -> kern_return_t;
    fn IOConnectCallStructMethod(connection: mach_port_t, selector: u32, inputStruct: *const c_void, inputStructCnt: usize, outputStruct: *const c_void, outputStructCnt: *const usize) -> kern_return_t;
}

static CONN: AtomicUsize = AtomicUsize::new(0);

#[ctor]
fn ctor() {
    unsafe {
        let c = CONN.load(Ordering::SeqCst) as io_connect_t;
        let mut result: kern_return_t;
        let iterator: io_iterator_t = ::std::mem::zeroed();

        let matching_dictionary = IOServiceMatching(c_str!(AppleSMC));
        result = IOServiceGetMatchingServices(kIOMasterPortDefault, matching_dictionary, &iterator);
        if result != kIOReturnSuccess {
            panic!("Error: IOServiceGetMatchingServices()");
        }

        let device = IOIteratorNext(&*iterator);
        IOObjectRelease(&*iterator);

        if device.is_null() {
            panic!("Error: no SMC found\n");
        }

        result = IOServiceOpen(&*device, mach_task_self(), 0, &c);
        IOObjectRelease(&*device);
        if result != kIOReturnSuccess {
            panic!("Error: IOServiceOpen()");
        }

        CONN.swap(c as usize, Ordering::SeqCst);
    }
}

#[dtor]
fn dtor() {
    let c = CONN.load(Ordering::SeqCst) as io_connect_t;
    unsafe { IOServiceClose(c) };
}

fn _strtoul(str: *const c_char, size: c_int, base: c_int) -> u32 {
    let mut total: u32 = 0;

    for i in 0..size {
        unsafe {
            if base == 16 {
                total += (*(((str as usize) + (i as usize)) as *const c_char) as u32) << (size - 1 - i) * 8;
            } else {
                total += (((*(((str as usize) + (i as usize)) as *const c_char) as u32) << (size - 1 - i) * 8) as c_uchar) as u32;
            }
        }
    }

    total
}

unsafe fn _ultostr(str: *mut c_char, val: u32) {
    *str = 0;
    libc::sprintf(str, c_str!("%c%c%c%c"),
        (val >> 24) as c_uint,
        (val >> 16) as c_uint,
        (val >> 8) as c_uint,
        val as c_uint);
}

unsafe fn smc_call(index: u32, input: &mut SMCKeyData_t, output: &mut SMCKeyData_t) -> kern_return_t {
    let c = CONN.load(Ordering::SeqCst) as io_connect_t;
    let input_size: usize = ::std::mem::size_of::<SMCKeyData_t>();
    let output_size: usize = ::std::mem::size_of::<SMCKeyData_t>();

    IOConnectCallStructMethod(c, index, input as *const SMCKeyData_t as *const c_void, input_size, output as *const SMCKeyData_t as *const c_void, &output_size)
}

unsafe fn read_key(key: *const c_char, val: &mut SMCVal_t) -> kern_return_t {
    let mut result: kern_return_t;
    let mut input: SMCKeyData_t = ::std::mem::zeroed();
    let mut output: SMCKeyData_t = ::std::mem::zeroed();

    input.key = _strtoul(key, 4, 16);
    input.data8 = SMC_CMD_READ_KEYINFO;

    result = smc_call(KERNEL_INDEX_SMC, &mut input, &mut output);

    if result != kIOReturnSuccess {
        return result;
    }

    val.dataSize = output.keyInfo.dataSize;
    _ultostr(&mut val.dataType[0] as *mut c_char, output.keyInfo.dataType);
    input.keyInfo.dataSize = val.dataSize;
    input.data8 = SMC_CMD_READ_BYTES;

    result = smc_call(KERNEL_INDEX_SMC, &mut input, &mut output);
    if result != kIOReturnSuccess {
        return result;
    }

    libc::memcpy(&mut val.bytes[0] as *mut c_char as *mut c_void, &mut output.bytes[0] as *mut c_char as *mut c_void, ::std::mem::size_of::<SMCBytes_t>());

    kIOReturnSuccess
}

fn temperature(key: *const c_char) -> Option<f64> {
    unsafe {
        let mut val: SMCVal_t = ::std::mem::zeroed();

        if read_key(key, &mut val) == kIOReturnSuccess {
            if val.dataSize > 0 {
                if libc::strcmp(&val.dataType[0] as *const c_char, DATATYPE_SP78) == 0 {
                    let value: f64 = ((val.bytes[0] as c_int) * 256 + ((val.bytes[1] as c_uchar) as c_int)) as f64;
                    return Some(value / 256.0);
                }
            }
        }

        None
    }
}

fn fan_rpm(key: *const c_char) -> Option<f64> {
    unsafe {
        let mut val: SMCVal_t = ::std::mem::zeroed();

        if read_key(key, &mut val) == kIOReturnSuccess {
            if val.dataSize > 0 {
                if libc::strcmp(&val.dataType[0] as *const c_char, DATATYPE_FPE2) == 0 {
                    return Some((u16::from_be(*(&val.bytes[0] as *const i8 as *const u16)) as f64) / 4.0);
                }
            }
        }

        None
    }
}

pub fn cpu_temp() -> f64 {
    temperature(SMC_KEY_CPU_TEMP).unwrap()
}

pub fn gpu_temp() -> Option<f64> {
    temperature(SMC_KEY_GPU_TEMP)
}

macro_rules! c_format {
    ($($arg:tt)*) => {
        CString::new(format!($($arg)*)).unwrap().into_raw()
    }
}

#[derive(Debug)]
pub struct Fan {
    pub name: String,
    pub min_speed: f64,
    pub actual_speed: f64,
    pub max_speed: f64,
}

impl std::fmt::Display for Fan {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Fan \"{}\" at {} RPM ({}%)", self.name, self.rpm(), self.percent())
    }
}

impl Fan {
    pub fn rpm(&self) -> f64 {
        let mut rpm = self.actual_speed - self.min_speed;
        if rpm < 0.0 {
            rpm = 0.0;
        }

        rpm
    }

    pub fn percent(&self) -> f64 {
        self.rpm() / (self.max_speed - self.min_speed) * 100.0
    }
}

type Fans = Vec<Fan>;

pub fn fans_rpm() -> Option<Fans> {
    let mut val: SMCVal_t = unsafe { ::std::mem::zeroed() };

    if unsafe { read_key(c_str!(FNum), &mut val) } != kIOReturnSuccess {
        return None;
    }

    let nfans = _strtoul(&val.bytes[0] as *const c_char, val.dataSize as c_int, 10) as usize;
    let mut res: Vec<Fan> = Vec::with_capacity(nfans);

    for i in 0..nfans {
        unsafe { libc::memset(&mut val as *mut SMCVal_t as *mut c_void, 0, ::std::mem::size_of::<SMCVal_t>()); }
        if unsafe { read_key(c_format!("F{}ID", i), &mut val) } != kIOReturnSuccess {
            return None;
        }

        let name = String::from(unsafe {
            CStr::from_ptr(((&val.bytes[0] as *const c_char as usize) + 4) as *mut c_char)
                .to_str().unwrap().trim()
        });

        let actual_speed = fan_rpm(c_format!("F{}Ac", i))?;
        let minimum_speed = fan_rpm(c_format!("F{}Mn", i))?;
        let maximum_speed = fan_rpm(c_format!("F{}Mx", i))?;

        res.push(Fan {
            name: name,
            min_speed: minimum_speed,
            actual_speed: actual_speed,
            max_speed: maximum_speed,
        });
    }

    Some(res)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use crate::{ cpu_temp, gpu_temp, fans_rpm };

        cpu_temp();
        gpu_temp();
        fans_rpm();
    }
}
