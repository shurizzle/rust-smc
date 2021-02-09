#[cfg(not(target_os = "macos"))]
compile_error!("SMC only works on macOS");

extern crate four_char_code;
extern crate libc;
#[macro_use]
extern crate lazy_static;

mod conversions;
mod sys;

use std::collections::HashMap;
use std::fmt;
use std::os::raw::c_void;
use std::sync::{Arc, Mutex};

use self::{conversions::*, sys::*};

use four_char_code::{four_char_code, FourCharCode};

use libc::{sysctl, CTL_HW};

#[derive(Default, Debug, Copy, Clone)]
pub struct SMCBytes([u8; 32]); // 32

// "ch8*", "char", "flag", "flt ", "fp1f", "fp6a", "fp79", "fp88", "fpe2", "hex_", "si16", "si8 ", "sp1e", "sp2d", "sp3c", "sp4b", "sp5a", "sp69", "sp78", "sp87", "ui16", "ui32", "ui8 ", "{alc", "{ali", "{alp", "{alv", "{fds", "{hdi", "{lim", "{lkb", "{lks", "{mss", "{rev"
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct DataType {
    pub id: FourCharCode,
    pub size: u32,
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
pub struct SMCKey {
    pub code: FourCharCode,
    pub info: DataType,
}

macro_rules! fcc_format {
    ( $fmt:literal, $( $args:expr ),+ ) => {
        Into::<FourCharCode>::into(format!($fmt, $($args),+))
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
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
    major: u8,
    minor: u8,
    build: u8,
    reserved: u8,
    release: u16,
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct SMCPLimitData {
    version: u16,
    length: u16,
    cpu_plimit: u32,
    gpu_plimit: u32,
    mem_plimit: u32,
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct SMCKeyInfoData {
    data_size: u32,
    data_type: FourCharCode,
    data_attributes: u8,
}

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct SMCParam {
    key: FourCharCode,
    vers: SMCVersion,
    p_limit_data: SMCPLimitData,
    key_info: SMCKeyInfoData,
    result: u8,
    status: u8,
    selector: SMCSelector,
    data32: u32,
    bytes: SMCBytes,
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

const SYS_IOKIT: kern_return_t = err_system!(0x38);
const SUB_IOKIT_COMMON: kern_return_t = err_sub!(0);

macro_rules! iokit_common_err {
    ( $err:literal ) => {
        SYS_IOKIT | SUB_IOKIT_COMMON | $err
    };
}

const KERN_SUCCESS: kern_return_t = 0;
#[allow(non_upper_case_globals)]
const kIOReturnSuccess: kern_return_t = KERN_SUCCESS;
#[allow(non_upper_case_globals)]
const kIOReturnNotPrivileged: kern_return_t = iokit_common_err!(0x2c1);

const MACH_PORT_NULL: mach_port_t = 0 as mach_port_t;
#[allow(non_upper_case_globals)]
const kIOMasterPortDefault: mach_port_t = MACH_PORT_NULL;

const TYPE_SP78: FourCharCode = four_char_code!("sp78");

const HW_PACKAGES: i32 = 125;
const HW_PHYSICALCPU: i32 = 101;

#[derive(Debug)]
pub enum SMCError {
    DriverNotFound,
    FailedToOpen,
    KeyNotFound(FourCharCode),
    NotPrivileged,
    UnsafeFanSpeed,
    Unknown(i32, u8),
    Sysctl(i32),
}

impl SMCError {
    pub fn code(&self) -> Option<FourCharCode> {
        match self {
            SMCError::KeyNotFound(code) => Some(*code),
            _ => None,
        }
    }

    pub fn io_result(&self) -> Option<i32> {
        match self {
            SMCError::Unknown(io_res, _) => Some(*io_res),
            _ => None,
        }
    }

    pub fn smc_result(&self) -> Option<u8> {
        match self {
            SMCError::Unknown(_, smc_res) => Some(*smc_res),
            _ => None,
        }
    }
}

impl fmt::Display for SMCError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SMCError::DriverNotFound => write!(f, "Driver not found."),
            SMCError::FailedToOpen => write!(f, "Failed to open driver."),
            SMCError::KeyNotFound(code) => write!(f, "Key {:?} not found.", code),
            SMCError::NotPrivileged => write!(f, "You do NOT have enough privileges."),
            SMCError::UnsafeFanSpeed => write!(f, "Fan speed is unsafe to be setted."),
            SMCError::Unknown(io_res, smc_res) => write!(
                f,
                "Unknown error: IOKit exited with code {} and SMC result {}.",
                io_res, smc_res
            ),
            SMCError::Sysctl(errno) => write!(f, "sysctl() call failed with errno {}.", errno),
        }
    }
}

impl std::error::Error for SMCError {
    fn description(&self) -> &str {
        "SMC error"
    }
}

macro_rules! sysctl_errno {
    () => {
        SMCError::Sysctl(::std::io::Error::last_os_error().raw_os_error().unwrap())
    };
}

fn get_cpus_number() -> Option<usize> {
    let mut mib: [i32; 2] = [CTL_HW, HW_PACKAGES];
    let mut num: u32 = 0;
    let mut len: usize = std::mem::size_of::<u32>();

    let res = unsafe {
        sysctl(
            &mut mib[0] as *mut _,
            2,
            &mut num as *mut _ as *mut c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if res == -1 {
        None
    } else {
        Some(num as usize)
    }
}

fn get_cores_number() -> Option<usize> {
    let mut mib: [i32; 2] = [CTL_HW, HW_PHYSICALCPU];
    let mut num: u32 = 0;
    let mut len: usize = std::mem::size_of::<u32>();

    let res = unsafe {
        sysctl(
            &mut mib[0] as *mut _,
            2,
            &mut num as *mut _ as *mut c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if res == -1 {
        None
    } else {
        Some(num as usize)
    }
}

struct SMCRepr(Mutex<io_connect_t>);

impl SMCRepr {
    fn new() -> Result<SMCRepr, SMCError> {
        let conn: io_connect_t = kIOMasterPortDefault;
        let result: kern_return_t;
        let device = unsafe {
            IOServiceGetMatchingService(
                kIOMasterPortDefault,
                IOServiceMatching(b"AppleSMC\0" as *const _),
            )
        };

        if device.is_null() {
            return Err(SMCError::DriverNotFound);
        }

        result = unsafe { IOServiceOpen(&mut *device, mach_task_self(), 0, &conn) };
        unsafe { IOObjectRelease(&mut *device) };
        if result != kIOReturnSuccess {
            return Err(SMCError::FailedToOpen);
        }

        Ok(SMCRepr(Mutex::new(conn)))
    }

    #[allow(non_upper_case_globals)]
    fn call_driver(&self, input: &SMCParam) -> Result<SMCParam, SMCError> {
        let mut output: SMCParam = Default::default();
        let input_size: usize = std::mem::size_of::<SMCParam>();
        let mut output_size: usize = std::mem::size_of::<SMCParam>();

        let conn = self.0.lock().unwrap();

        let result = unsafe {
            IOConnectCallStructMethod(
                *conn,
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

    fn read_data<T>(&self, key: SMCKey) -> Result<T, SMCError>
    where
        T: SMCType,
    {
        let mut input: SMCParam = Default::default();
        input.key = key.code;
        input.key_info.data_size = key.info.size;
        input.selector = SMCSelector::ReadKey;

        let output = self.call_driver(&input)?;

        Ok(SMCType::from_smc(key.info, output.bytes))
    }

    fn write_data<T>(&self, key: SMCKey, data: T) -> Result<(), SMCError>
    where
        T: SMCType,
    {
        let mut input: SMCParam = Default::default();
        input.key = key.code;
        input.bytes = SMCType::to_smc(&data, key.info);
        input.key_info.data_size = key.info.size;
        input.selector = SMCSelector::WriteKey;

        self.call_driver(&input)?;

        Ok(())
    }

    fn key_information(&self, key: FourCharCode) -> Result<DataType, SMCError> {
        let mut input: SMCParam = Default::default();
        input.key = key;
        input.selector = SMCSelector::GetKeyInfo;

        let output = self.call_driver(&input)?;

        Ok(DataType {
            id: output.key_info.data_type,
            size: output.key_info.data_size,
        })
    }

    fn read_key<T>(&self, code: FourCharCode) -> Result<T, SMCError>
    where
        T: SMCType,
    {
        let info = self.key_information(code)?;
        self.read_data(SMCKey { code, info })
    }

    fn write_key<T>(&self, code: FourCharCode, data: T) -> Result<(), SMCError>
    where
        T: SMCType,
    {
        let info = self.key_information(code)?;
        self.write_data(SMCKey { code, info }, data)
    }

    fn key_information_at_index(&self, index: u32) -> Result<FourCharCode, SMCError> {
        let mut input: SMCParam = Default::default();
        input.selector = SMCSelector::GetKeyFromIndex;
        input.data32 = index;

        let output = self.call_driver(&input)?;

        Ok(output.key)
    }
}

impl Drop for SMCRepr {
    fn drop(&mut self) {
        let conn = self.0.lock().unwrap();
        unsafe { IOServiceClose(*conn) };
    }
}

unsafe impl Send for SMCRepr {}
unsafe impl Sync for SMCRepr {}

lazy_static! {
    static ref SHARED: Mutex<Option<Arc<SMCRepr>>> = Mutex::new(None);
}

pub struct Fan {
    smc_repr: Arc<SMCRepr>,
    id: u32,
    name: String,
}

impl fmt::Debug for Fan {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Fan")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}

impl Clone for Fan {
    fn clone(&self) -> Fan {
        Fan {
            smc_repr: self.smc_repr.clone(),
            id: self.id,
            name: self.name.clone(),
        }
    }
}

impl Fan {
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn min_speed(&self) -> Result<f64, SMCError> {
        self.smc_repr.read_key(fcc_format!("F{}Mn", self.id))
    }

    pub fn max_speed(&self) -> Result<f64, SMCError> {
        self.smc_repr.read_key(fcc_format!("F{}Mx", self.id))
    }

    pub fn current_speed(&self) -> Result<f64, SMCError> {
        self.smc_repr.read_key(fcc_format!("F{}Ac", self.id))
    }

    pub fn rpm(&self) -> Result<f64, SMCError> {
        let mut rpm = self.current_speed()? - self.min_speed()?;
        if rpm < 0.0 {
            rpm = 0.0;
        }

        Ok(rpm)
    }

    pub fn is_managed(&self) -> Result<bool, SMCError> {
        let bitmask: u16 = self.smc_repr.read_key(four_char_code!("FS! "))?;
        Ok(bitmask & (1_u16 << (self.id as u16)) == 0)
    }

    pub fn set_managed(&self, what: bool) -> Result<(), SMCError> {
        let bitmask: u16 = self.smc_repr.read_key(four_char_code!("FS! "))?;
        let mask = 1_u16 << (self.id as u16);
        let new: u16 = if what {
            bitmask & !mask
        } else {
            bitmask | mask
        };

        if bitmask != new {
            self.smc_repr.write_key(four_char_code!("FS! "), new)
        } else {
            Ok(())
        }
    }

    pub fn set_min_speed(&self, speed: f64) -> Result<(), SMCError> {
        let max = self.max_speed()?;
        if speed <= 0.0 || speed > max {
            Err(SMCError::UnsafeFanSpeed)
        } else {
            self.smc_repr
                .write_key(fcc_format!("F{}Mn", self.id), speed)
        }
    }

    pub fn set_current_speed(&self, speed: f64) -> Result<(), SMCError> {
        let min = self.min_speed()?;
        let max = self.max_speed()?;
        if speed <= min || speed > max {
            Err(SMCError::UnsafeFanSpeed)
        } else {
            self.set_managed(false)?;
            self.smc_repr
                .write_key(fcc_format!("F{}Tg", self.id), speed)
        }
    }

    pub fn percent(&self) -> Result<f64, SMCError> {
        let current = self.current_speed()?;
        let min = self.min_speed()?;
        let max = self.max_speed()?;

        let rpm = current - min;
        let rpm = if rpm < 0.0 { 0.0 } else { rpm };

        Ok(rpm / (max - min) * 100.0)
    }
}

unsafe impl Send for Fan {}
unsafe impl Sync for Fan {}

pub struct SMC(Arc<SMCRepr>);

impl SMC {
    pub fn new() -> Result<SMC, SMCError> {
        Ok(SMC(Arc::new(SMCRepr::new()?)))
    }

    pub fn shared() -> Result<SMC, SMCError> {
        let mut shared = SHARED.lock().unwrap();
        match (*shared).as_ref() {
            None => {
                let smc = Arc::new(SMCRepr::new()?);
                let res = smc.clone();
                *shared = Some(smc);
                Ok(SMC(res))
            }
            Some(shared) => Ok(SMC(shared.clone())),
        }
    }

    fn _keys_len(&self) -> Result<u32, SMCError> {
        self.0.read_key(four_char_code!("#KEY"))
    }

    pub fn keys_len(&self) -> Result<usize, SMCError> {
        Ok(self._keys_len()? as usize)
    }

    pub fn keys(&self) -> Result<Vec<FourCharCode>, SMCError> {
        let len = self._keys_len()?;
        let mut res: Vec<FourCharCode> = Vec::with_capacity(len as usize);

        for i in 0..len {
            res.push(self.0.key_information_at_index(i)?);
        }

        Ok(res)
    }

    pub fn smc_keys(&self) -> Result<Vec<SMCKey>, SMCError> {
        let len = self._keys_len()?;
        let mut res: Vec<SMCKey> = Vec::with_capacity(len as usize);

        for i in 0..len {
            let key = self.0.key_information_at_index(i)?;
            let info = self.0.key_information(key)?;
            res.push(SMCKey { code: key, info });
        }

        Ok(res)
    }

    pub fn fans_len(&self) -> Result<usize, SMCError> {
        Ok(usize::from(self.0.read_key::<u8>(four_char_code!("FNum"))?))
    }

    pub fn fan(&self, id: u32) -> Result<Fan, SMCError> {
        let res: RawFan = self.0.read_key(fcc_format!("F{}ID", id))?;

        Ok(Fan {
            smc_repr: self.0.clone(),
            id,
            name: res.name,
        })
    }

    pub fn fans(&self) -> Result<Vec<Fan>, SMCError> {
        let len = self.fans_len()?;
        let mut res: Vec<Fan> = Vec::with_capacity(len);

        for i in 0..len {
            res.push(self.fan(i as u32)?);
        }

        Ok(res)
    }

    pub fn is_optical_disk_drive_full(&self) -> Result<bool, SMCError> {
        self.0.read_key(four_char_code!("MSDI"))
    }

    pub fn all_temperature_sensors_keys(&self) -> Result<Vec<FourCharCode>, SMCError> {
        Ok(self
            .smc_keys()?
            .into_iter()
            .filter_map(|k| {
                if k.code.to_string().starts_with("T") && k.info.id == TYPE_SP78 {
                    Some(k.code)
                } else {
                    None
                }
            })
            .collect())
    }

    pub fn all_temperature_sensors(&self) -> Result<HashMap<FourCharCode, f64>, SMCError> {
        let keys = self.all_temperature_sensors_keys()?;
        let mut res = HashMap::with_capacity(keys.len());

        for key in keys.into_iter() {
            res.insert(key, self.0.read_key(key)?);
        }

        Ok(res)
    }

    pub fn temperature(&self, key: FourCharCode) -> Result<f64, SMCError> {
        if key.to_string().starts_with("T") {
            let info = self.0.key_information(key)?;

            if info.id == TYPE_SP78 {
                self.0.read_key(key)
            } else {
                Err(SMCError::KeyNotFound(key))
            }
        } else {
            Err(SMCError::KeyNotFound(key))
        }
    }

    pub fn cpu_temperature(&self, id: u8) -> Result<f64, SMCError> {
        self.temperature(fcc_format!("TC{}C", id))
    }

    pub fn cpus_temperature(&self) -> Result<Vec<f64>, SMCError> {
        let cores = match get_cores_number() {
            Some(x) => x as u8,
            None => return Err(sysctl_errno!()),
        };

        let mut res: Vec<f64> = Vec::with_capacity(usize::from(cores));

        for i in 0..cores {
            res.push(self.cpu_temperature(i)?);
        }

        Ok(res)
    }

    pub fn package_temperature(&self, id: u8) -> Result<Vec<f64>, SMCError> {
        let cpusno = match get_cpus_number() {
            Some(x) => x as u8,
            None => return Err(sysctl_errno!()),
        };

        let cores = match get_cores_number() {
            Some(x) => x as u8,
            None => return Err(sysctl_errno!()),
        };

        let cpc = cores / cpusno;
        let start = cpc * id;
        let stop = start + cpc;

        let mut res: Vec<f64> = Vec::with_capacity(usize::from(cpc));

        for i in start..stop {
            res.push(self.cpu_temperature(i)?);
        }

        Ok(res)
    }

    pub fn packages_temperature(&self) -> Result<Vec<Vec<f64>>, SMCError> {
        let cpusno = match get_cpus_number() {
            Some(x) => x as u8,
            None => return Err(sysctl_errno!()),
        };

        let mut res: Vec<Vec<f64>> = Vec::with_capacity(usize::from(cpusno));

        for i in 0..cpusno {
            res.push(self.package_temperature(i)?);
        }

        Ok(res)
    }

    pub fn gpu_temperature(&self, id: u8) -> Result<f64, SMCError> {
        self.temperature(fcc_format!("FG{}C", id))
    }

    pub fn gpus_temperature(&self) -> Result<Vec<f64>, SMCError> {
        let mut res: Vec<f64> = Vec::new();
        let mut idx: u8 = 0;

        loop {
            match self.gpu_temperature(idx) {
                Ok(temp) => {
                    res.push(temp);
                }
                Err(SMCError::KeyNotFound(_)) => {
                    break;
                }
                Err(err) => {
                    return Err(err);
                }
            }
            idx += 1;
        }

        Ok(res)
    }
}

impl Clone for SMC {
    fn clone(&self) -> SMC {
        SMC(self.0.clone())
    }
}
