use core::{borrow::Borrow, fmt, ops::Deref};

use four_char_code::{four_char_code as fcc, FourCharCode};

use crate::SMCVal;

pub(crate) const TYPE_FLAG: FourCharCode = fcc!("flag");
pub(crate) const TYPE_I8: FourCharCode = fcc!("si8 ");
pub(crate) const TYPE_U8: FourCharCode = fcc!("ui8 ");
pub(crate) const TYPE_I16: FourCharCode = fcc!("si16");
pub(crate) const TYPE_U16: FourCharCode = fcc!("ui16");
pub(crate) const TYPE_I32: FourCharCode = fcc!("si32");
pub(crate) const TYPE_U32: FourCharCode = fcc!("ui32");
pub(crate) const TYPE_FLT: FourCharCode = fcc!("flt ");
pub(crate) const TYPE_FPE2: FourCharCode = fcc!("fpe2");
pub(crate) const TYPE_SP78: FourCharCode = fcc!("sp78");

pub trait IntoSMC {
    fn into_smc(self, param: &mut SMCVal) -> Option<()>;
}

pub trait FromSMC: Sized {
    fn from_smc(param: SMCVal) -> Option<Self>;
}

impl FromSMC for SMCVal {
    fn from_smc(param: SMCVal) -> Option<Self> {
        Some(param)
    }
}

impl FromSMC for bool {
    fn from_smc(param: SMCVal) -> Option<Self> {
        if param.r#type != TYPE_FLAG || param.len() != 1 {
            return None;
        }

        Some(unsafe { *param.data().get_unchecked(0) != 0 })
    }
}

impl FromSMC for u8 {
    fn from_smc(param: SMCVal) -> Option<Self> {
        if param.r#type != TYPE_U8 || param.len() != 1 {
            return None;
        }

        Some(unsafe { *param.data().get_unchecked(0) })
    }
}

impl FromSMC for i8 {
    fn from_smc(param: SMCVal) -> Option<Self> {
        if param.r#type != TYPE_I8 || param.len() != 1 {
            return None;
        }

        Some(unsafe { *param.data().as_ptr().cast::<i8>() })
    }
}

impl FromSMC for u16 {
    fn from_smc(param: SMCVal) -> Option<Self> {
        unsafe {
            match (param.r#type, param.len()) {
                (TYPE_U8, 1) => Some(*param.data().get_unchecked(0) as u16),
                (TYPE_U16, 2) => Some(u16::from_be(*param.data().as_ptr().cast())),
                _ => None,
            }
        }
    }
}

impl FromSMC for i16 {
    fn from_smc(param: SMCVal) -> Option<Self> {
        unsafe {
            match (param.r#type, param.len()) {
                (TYPE_U8, 1) => Some(*param.data().get_unchecked(0) as i16),
                (TYPE_I8, 1) => Some(i8::from_be(*param.data().as_ptr().cast()) as i16),
                (TYPE_I16, 2) => Some(i16::from_be(*param.data().as_ptr().cast())),
                _ => None,
            }
        }
    }
}

impl FromSMC for u32 {
    fn from_smc(param: SMCVal) -> Option<Self> {
        unsafe {
            match (param.r#type, param.len()) {
                (TYPE_U8, 1) => Some(*param.data().get_unchecked(0) as u32),
                (TYPE_U16, 2) => Some(u16::from_be(*param.data().as_ptr().cast()) as u32),
                (TYPE_U32, 4) => Some(u32::from_be(*param.data().as_ptr().cast())),
                _ => None,
            }
        }
    }
}

impl FromSMC for i32 {
    fn from_smc(param: SMCVal) -> Option<Self> {
        unsafe {
            match (param.r#type, param.len()) {
                (TYPE_U8, 1) => Some(*param.data().get_unchecked(0) as i32),
                (TYPE_I8, 1) => Some((*param.data().as_ptr().cast::<i8>()) as i32),
                (TYPE_U16, 2) => Some(u16::from_be(*param.data().as_ptr().cast()) as i32),
                (TYPE_I16, 2) => Some(i16::from_be(*param.data().as_ptr().cast()) as i32),
                (TYPE_I32, 4) => Some(i32::from_be(*param.data().as_ptr().cast())),
                _ => None,
            }
        }
    }
}

impl FromSMC for i64 {
    fn from_smc(param: SMCVal) -> Option<Self> {
        unsafe {
            match (param.r#type, param.len()) {
                (TYPE_U8, 1) => Some(*param.data().get_unchecked(0) as i64),
                (TYPE_I8, 1) => Some((*param.data().as_ptr().cast::<i8>()) as i64),
                (TYPE_U16, 2) => Some(u16::from_be(*param.data().as_ptr().cast()) as i64),
                (TYPE_I16, 2) => Some(i16::from_be(*param.data().as_ptr().cast()) as i64),
                (TYPE_U32, 4) => Some(u32::from_be(*param.data().as_ptr().cast()) as i64),
                (TYPE_I32, 4) => Some(i32::from_be(*param.data().as_ptr().cast()) as i64),
                _ => None,
            }
        }
    }
}

impl FromSMC for f32 {
    fn from_smc(param: SMCVal) -> Option<Self> {
        unsafe {
            match (param.r#type, param.len()) {
                (TYPE_FPE2, 2) => Some(u16::from_be(*param.data().as_ptr().cast()) as f32 / 4.0),
                (TYPE_SP78, 2) => Some(i16::from_be(*param.data().as_ptr().cast()) as f32 / 256.0),
                (TYPE_FLT, 4) => Some(*param.data().as_ptr().cast::<f32>()),
                _ => None,
            }
        }
    }
}

#[derive(Default, Clone, Copy)]
#[repr(transparent)]
pub struct UMax10(u8);

impl UMax10 {
    pub const fn new(value: u8) -> Option<Self> {
        if value > 10 {
            None
        } else {
            Some(Self(value))
        }
    }

    pub const unsafe fn new_unchecked(value: u8) -> Self {
        Self(value)
    }
}

impl Deref for UMax10 {
    type Target = u8;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<u8> for UMax10 {
    #[inline]
    fn as_ref(&self) -> &u8 {
        self
    }
}

impl Borrow<u8> for UMax10 {
    #[inline]
    fn borrow(&self) -> &u8 {
        self
    }
}

impl fmt::Display for UMax10 {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for UMax10 {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct GreaterThan10;

impl fmt::Display for GreaterThan10 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "number is greater than 10")
    }
}

#[cfg(feature = "std")]
impl ::std::error::Error for GreaterThan10 {}

macro_rules! def_max10_try_from {
    ($($t:ty),+ $(,)?) => {
        $(
            impl TryFrom<$t> for UMax10 {
                type Error = GreaterThan10;

                fn try_from(value: $t) -> Result<Self, Self::Error> {
                    if value > 10 {
                        Err(GreaterThan10)
                    } else {
                        Ok(Self(value as u8))
                    }
                }
            }
        )+
    };
}

def_max10_try_from!(u8, u16, u32, u64, u128);

impl FromSMC for UMax10 {
    fn from_smc(param: SMCVal) -> Option<Self> {
        (u32::from_smc(param)?).try_into().ok()
    }
}

#[derive(Default, Clone, Copy)]
#[repr(transparent)]
pub struct OneDigit(u8);

impl OneDigit {
    pub const fn new(value: u8) -> Option<Self> {
        if value > 9 {
            None
        } else {
            Some(Self(value))
        }
    }

    pub const unsafe fn new_unchecked(value: u8) -> Self {
        Self(value)
    }
}

impl Deref for OneDigit {
    type Target = u8;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<u8> for OneDigit {
    #[inline]
    fn as_ref(&self) -> &u8 {
        self
    }
}

impl Borrow<u8> for OneDigit {
    #[inline]
    fn borrow(&self) -> &u8 {
        self
    }
}

impl fmt::Display for OneDigit {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for OneDigit {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct MoreDigits;

impl fmt::Display for MoreDigits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "number contains more digits")
    }
}

#[cfg(feature = "std")]
impl ::std::error::Error for MoreDigits {}

macro_rules! def_one_digit_try_from {
    ($($t:ty),+ $(,)?) => {
        $(
            impl TryFrom<$t> for OneDigit {
                type Error = MoreDigits;

                fn try_from(value: $t) -> Result<Self, Self::Error> {
                    if value > 9 {
                        Err(MoreDigits)
                    } else {
                        Ok(Self(value as u8))
                    }
                }
            }
        )+
    };
}

def_one_digit_try_from!(u8, u16, u32, u64, u128);

impl FromSMC for OneDigit {
    fn from_smc(param: SMCVal) -> Option<Self> {
        (u32::from_smc(param)?).try_into().ok()
    }
}
