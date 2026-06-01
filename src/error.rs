use core::fmt;

use four_char_code::FourCharCode;

use crate::SMCVal;

/// Errors that can occur when interacting with the SMC.
#[derive(Debug)]
pub enum SMCError {
    /// The SMC kernel extension driver was not found on the system.
    DriverNotFound,
    /// Failed to open a connection to the SMC driver.
    Open,
    /// The given key string is not a valid FourCharCode.
    InvalidKey(four_char_code::FccConversionError),
    /// The specified FourCharCode key does not exist in the SMC.
    KeyNotFound(FourCharCode),
    /// Insufficient privileges; the operation likely requires root.
    NotPrivileged,
    /// Failed to convert an SMC value into the requested Rust type.
    TryFrom(SMCVal),
    /// Failed to convert a value into an SMC value.
    TryInto,
    /// Unknown error from the IOKit/SMC interface.
    ///
    /// Contains the IOKit return code and the SMC result code respectively.
    Unknown(i32, u8),
    /// A `sysctl()` call failed. Wraps the errno.
    Sysctl(i32),
}

impl From<four_char_code::FccConversionError> for SMCError {
    #[inline]
    fn from(value: four_char_code::FccConversionError) -> Self {
        Self::InvalidKey(value)
    }
}

impl SMCError {
    /// Returns the `FourCharCode` of the missing key if the error is `KeyNotFound`.
    pub fn code(&self) -> Option<FourCharCode> {
        match self {
            SMCError::KeyNotFound(code) => Some(*code),
            _ => None,
        }
    }

    /// Returns the IOKit return code if the error is `Unknown`.
    pub fn io_result(&self) -> Option<i32> {
        match self {
            SMCError::Unknown(io_res, _) => Some(*io_res),
            _ => None,
        }
    }

    /// Returns the SMC result code if the error is `Unknown`.
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
            SMCError::Open => write!(f, "Failed to open driver."),
            SMCError::InvalidKey(err) => fmt::Display::fmt(err, f),
            SMCError::KeyNotFound(code) => write!(f, "Key {:?} not found.", code),
            SMCError::NotPrivileged => write!(f, "You do NOT have enough privileges."),
            SMCError::TryFrom(_) => write!(f, "Invalid conversion from smc value"),
            SMCError::TryInto => write!(f, "Invalid conversion into smc value"),
            SMCError::Unknown(io_res, smc_res) => write!(
                f,
                "Unknown error: IOKit exited with code {} and SMC result {}.",
                io_res, smc_res
            ),
            SMCError::Sysctl(errno) => write!(f, "sysctl() call failed with errno {}.", errno),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SMCError {
    fn description(&self) -> &str {
        "SMC error"
    }
}
