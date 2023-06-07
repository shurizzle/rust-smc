#![cfg(target_os = "macos")]
#![cfg_attr(not(feature = "std"), no_std)]

use core::mem::MaybeUninit;

use four_char_code::{four_char_code, FourCharCode};
use libc::c_void;
use sys::{io_connect_t, kIOReturnNotPrivileged, IOConnectCallStructMethod, IOServiceClose};

mod error;
mod fans;
mod keys;
pub(crate) mod sys;
mod types;
pub mod util;

pub use error::SMCError;
pub use fans::*;
pub use keys::Keys;
pub use types::*;

use crate::sys::{
    kIOMasterPortDefault, kIOReturnSuccess, IOObjectRelease, IOServiceGetMatchingService,
    IOServiceMatching, IOServiceOpen,
};

pub type Result<T> = core::result::Result<T, SMCError>;

// "ch8*", "char", "flag", "flt ", "fp1f", "fp6a", "fp79", "fp88", "fpe2", "hex_", "si16", "si8 ", "sp1e", "sp2d", "sp3c", "sp4b", "sp5a", "sp69", "sp78", "sp87", "ui16", "ui32", "ui8 ", "{alc", "{ali", "{alp", "{alv", "{fds", "{hdi", "{lim", "{lkb", "{lks", "{mss", "{rev"
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct DataType {
    pub id: FourCharCode,
    pub size: u32,
}

#[derive(Default, Debug, Copy, Clone)]
struct SMCKey {
    pub code: FourCharCode,
    pub info: DataType,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
#[non_exhaustive]
enum SMCSelector {
    Unknown = 0,
    // HandleYPCEvent = 2,
    ReadKey = 5,
    WriteKey = 6,
    GetKeyFromIndex = 8,
    GetKeyInfo = 9,
}

impl Default for SMCSelector {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct SMCVersion {
    pub major: u8,
    pub minor: u8,
    pub build: u8,
    pub reserved: u8,
    pub release: u16,
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct SMCPLimitData {
    pub version: u16,
    pub length: u16,
    pub cpu_plimit: u32,
    pub gpu_plimit: u32,
    pub mem_plimit: u32,
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct SMCKeyInfoData {
    pub data_size: u32,
    pub data_type: FourCharCode,
    pub data_attributes: u8,
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct SMCParam {
    pub key: FourCharCode,
    pub vers: SMCVersion,
    pub p_limit_data: SMCPLimitData,
    pub key_info: SMCKeyInfoData,
    pub result: u8,
    pub status: u8,
    pub selector: SMCSelector,
    pub data32: u32,
    pub bytes: [u8; 32],
}

#[derive(Default, Debug, Copy, Clone)]
pub struct SMCVal {
    pub r#type: FourCharCode,
    size: usize,
    data: [u8; 32],
}

impl SMCVal {
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn data(&self) -> &[u8] {
        unsafe { self.data.get_unchecked(..self.size) }
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut [u8] {
        unsafe { self.data.get_unchecked_mut(..self.size) }
    }
}

pub struct SMC(io_connect_t);

impl SMC {
    pub fn new() -> Result<Self> {
        unsafe {
            let device = IOServiceGetMatchingService(
                kIOMasterPortDefault,
                IOServiceMatching(b"AppleSMC\0" as *const _),
            );

            if device.is_null() {
                return Err(SMCError::DriverNotFound);
            }

            let mut conn = MaybeUninit::<io_connect_t>::uninit();
            let result = IOServiceOpen(&mut *device, libc::mach_task_self(), 0, conn.as_mut_ptr());
            IOObjectRelease(&mut *device);
            if result != kIOReturnSuccess {
                return Err(SMCError::Open);
            }

            Ok(Self(conn.assume_init()))
        }
    }

    #[allow(non_upper_case_globals, non_snake_case)]
    unsafe fn call_driver(&self, input: &SMCParam) -> Result<SMCParam> {
        let mut output: SMCParam = Default::default();
        let input_size: usize = core::mem::size_of::<SMCParam>();
        let mut output_size: usize = core::mem::size_of::<SMCParam>();

        let result = unsafe {
            IOConnectCallStructMethod(
                self.0,
                2,
                input as *const _ as *const c_void,
                input_size,
                &mut output as *mut _ as *mut c_void,
                &mut output_size,
            )
        };

        match (result, output.result) {
            (kIOReturnSuccess, 0) => Ok(output),
            (kIOReturnSuccess, 132) => Err(SMCError::KeyNotFound(input.key)),
            (kIOReturnNotPrivileged, _) => Err(SMCError::NotPrivileged),
            _ => Err(SMCError::Unknown(result, output.result)),
        }
    }

    pub fn key_info(&self, key: FourCharCode) -> Result<DataType> {
        let mut output = unsafe {
            self.call_driver(&SMCParam {
                key,
                selector: SMCSelector::GetKeyInfo,
                ..Default::default()
            })?
        };
        output.key_info.data_type.normalize();
        Ok(DataType {
            id: output.key_info.data_type,
            size: output.key_info.data_size,
        })
    }

    fn read_data<T>(&self, key: SMCKey) -> Result<T>
    where
        T: FromSMC,
    {
        let val = unsafe {
            let output = self.call_driver(&SMCParam {
                key: key.code,
                selector: SMCSelector::ReadKey,
                key_info: SMCKeyInfoData {
                    data_size: key.info.size,
                    ..Default::default()
                },
                ..Default::default()
            })?;
            SMCVal {
                r#type: key.info.id,
                size: key.info.size as usize,
                data: output.bytes,
            }
        };

        T::from_smc(val).map_or(Err(SMCError::TryFrom(val)), Ok)
    }

    fn write_data<T>(&mut self, key: SMCKey, val: T) -> Result<()>
    where
        T: IntoSMC,
    {
        let mut res = SMCVal {
            r#type: key.info.id,
            size: key.info.size as usize,
            ..Default::default()
        };
        if T::into_smc(val, &mut res).is_none() {
            return Err(SMCError::TryInto);
        }

        unsafe {
            self.call_driver(&SMCParam {
                key: key.code,
                key_info: SMCKeyInfoData {
                    data_size: key.info.size,
                    ..Default::default()
                },
                selector: SMCSelector::WriteKey,
                bytes: res.data,
                ..Default::default()
            })?
        };
        Ok(())
    }

    pub fn read_key<T>(&self, key: FourCharCode) -> Result<T>
    where
        T: FromSMC,
    {
        let data_type = self.key_info(key)?;
        self.read_data(SMCKey {
            code: key,
            info: data_type,
        })
    }

    pub unsafe fn write_key<T>(&mut self, key: FourCharCode, val: T) -> Result<()>
    where
        T: IntoSMC,
    {
        let data_type = self.key_info(key)?;
        self.write_data(
            SMCKey {
                code: key,
                info: data_type,
            },
            val,
        )
    }

    pub fn get_key(&self, index: u32) -> Result<FourCharCode> {
        unsafe {
            self.call_driver(&SMCParam {
                selector: SMCSelector::GetKeyFromIndex,
                data32: index,
                ..Default::default()
            })
            .map(|mut out| {
                out.key.normalize();
                out.key
            })
        }
    }

    #[inline]
    pub fn is_optical_disk_drive_full(&self) -> Result<bool> {
        self.read_key(four_char_code!("MSDI"))
    }
}

impl Drop for SMC {
    fn drop(&mut self) {
        unsafe { IOServiceClose(self.0) };
    }
}
