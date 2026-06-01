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

/// Conversion into an SMC value.
pub trait IntoSMC {
    /// Writes `self` into the given [`SMCVal`].
    fn into_smc(self, param: &mut SMCVal) -> Option<()>;
}

/// Conversion from an SMC value.
pub trait FromSMC: Sized {
    /// Constructs `Self` from the given [`SMCVal`].
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

/// A `u8` constrained to `0..=10`.
#[derive(Default, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct UMax10(u8);

impl UMax10 {
    /// Creates a `UMax10` if `value <= 10`, otherwise returns `None`.
    pub const fn new(value: u8) -> Option<Self> {
        if value > 10 {
            None
        } else {
            Some(Self(value))
        }
    }

    /// Creates a `UMax10` without checking that `value <= 10`.
    ///
    /// # Safety
    /// The caller must ensure `value <= 10`.
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

/// Error type indicating a value exceeded the maximum of 10.
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

/// A `u8` constrained to `0..=9` (one decimal digit).
#[derive(Default, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct OneDigit(u8);

impl OneDigit {
    /// Creates a `OneDigit` if `value <= 9`, otherwise returns `None`.
    pub const fn new(value: u8) -> Option<Self> {
        if value > 9 {
            None
        } else {
            Some(Self(value))
        }
    }

    /// Creates a `OneDigit` without checking that `value <= 9`.
    ///
    /// # Safety
    /// The caller must ensure `value <= 9`.
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

/// Error type indicating a value has more digits than expected.
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

#[cfg(test)]
    mod tests {
        use super::*;
        use core::convert::TryFrom;
        use four_char_code::FourCharCode;
        use std::format;

    fn make_val(r#type: FourCharCode, data: &[u8]) -> SMCVal {
        let mut val = SMCVal { r#type, size: data.len(), data: [0; 32] };
        val.data[..data.len()].copy_from_slice(data);
        val
    }

    mod from_smc {
        use super::*;

        #[test]
        fn bool_false() {
            let v = make_val(TYPE_FLAG, &[0]);
            assert_eq!(bool::from_smc(v), Some(false));
        }

        #[test]
        fn bool_true_nonzero() {
            let v = make_val(TYPE_FLAG, &[1]);
            assert_eq!(bool::from_smc(v), Some(true));
            let v = make_val(TYPE_FLAG, &[42]);
            assert_eq!(bool::from_smc(v), Some(true));
        }

        #[test]
        fn bool_wrong_type() {
            let v = make_val(TYPE_U8, &[0]);
            assert_eq!(bool::from_smc(v), None);
        }

        #[test]
        fn bool_wrong_size() {
            let v = make_val(TYPE_FLAG, &[0, 0]);
            assert_eq!(bool::from_smc(v), None);
        }

        #[test]
        fn u8_ok() {
            let v = make_val(TYPE_U8, &[42]);
            assert_eq!(u8::from_smc(v), Some(42));
        }

        #[test]
        fn u8_wrong_type() {
            let v = make_val(TYPE_FLAG, &[0]);
            assert_eq!(u8::from_smc(v), None);
        }

        #[test]
        fn u8_wrong_size() {
            let v = make_val(TYPE_U8, &[1, 2]);
            assert_eq!(u8::from_smc(v), None);
        }

        #[test]
        fn i8_ok_positive() {
            let v = make_val(TYPE_I8, &[42]);
            assert_eq!(i8::from_smc(v), Some(42i8));
        }

        #[test]
        fn i8_ok_negative() {
            let v = make_val(TYPE_I8, &[0xFE]);
            assert_eq!(i8::from_smc(v), Some(-2i8));
        }

        #[test]
        fn i8_wrong_type() {
            let v = make_val(TYPE_U8, &[0]);
            assert_eq!(i8::from_smc(v), None);
        }

        #[test]
        fn u16_direct() {
            let v = make_val(TYPE_U16, &[0x01, 0x02]);
            assert_eq!(u16::from_smc(v), Some(0x0102));
        }

        #[test]
        fn u16_widened_from_u8() {
            let v = make_val(TYPE_U8, &[0x2A]);
            assert_eq!(u16::from_smc(v), Some(0x2A));
        }

        #[test]
        fn u16_wrong_type() {
            let v = make_val(TYPE_I16, &[0, 0]);
            assert_eq!(u16::from_smc(v), None);
        }

        #[test]
        fn i16_direct() {
            let v = make_val(TYPE_I16, &[0xFF, 0xFE]);
            assert_eq!(i16::from_smc(v), Some(-2i16));
        }

        #[test]
        fn i16_widened_u8() {
            let v = make_val(TYPE_U8, &[0x2A]);
            assert_eq!(i16::from_smc(v), Some(0x2A));
        }

        #[test]
        fn i16_widened_i8_negative() {
            let v = make_val(TYPE_I8, &[0xFE]);
            assert_eq!(i16::from_smc(v), Some(-2i16));
        }

        #[test]
        fn u32_direct() {
            let v = make_val(TYPE_U32, &[0xDE, 0xAD, 0xBE, 0xEF]);
            assert_eq!(u32::from_smc(v), Some(0xDEAD_BEEF));
        }

        #[test]
        fn u32_widened_u8() {
            let v = make_val(TYPE_U8, &[0x2A]);
            assert_eq!(u32::from_smc(v), Some(0x2A));
        }

        #[test]
        fn u32_widened_u16() {
            let v = make_val(TYPE_U16, &[0x01, 0x02]);
            assert_eq!(u32::from_smc(v), Some(0x0102));
        }

        #[test]
        fn i32_direct_negative() {
            let v = make_val(TYPE_I32, &[0xFF, 0xFF, 0xFF, 0xFE]);
            assert_eq!(i32::from_smc(v), Some(-2i32));
        }

        #[test]
        fn i32_direct_positive() {
            let v = make_val(TYPE_I32, &[0x00, 0x00, 0x00, 0x2A]);
            assert_eq!(i32::from_smc(v), Some(42i32));
        }

        #[test]
        fn i32_widened_u8() {
            let v = make_val(TYPE_U8, &[0x2A]);
            assert_eq!(i32::from_smc(v), Some(42i32));
        }

        #[test]
        fn i32_widened_i8() {
            let v = make_val(TYPE_I8, &[0xFE]);
            assert_eq!(i32::from_smc(v), Some(-2i32));
        }

        #[test]
        fn i32_widened_u16() {
            let v = make_val(TYPE_U16, &[0x01, 0x02]);
            assert_eq!(i32::from_smc(v), Some(0x0102i32));
        }

        #[test]
        fn i32_widened_i16() {
            let v = make_val(TYPE_I16, &[0xFF, 0xFE]);
            assert_eq!(i32::from_smc(v), Some(-2i32));
        }

        #[test]
        fn i64_widened_u32() {
            let v = make_val(TYPE_U32, &[0xDE, 0xAD, 0xBE, 0xEF]);
            assert_eq!(i64::from_smc(v), Some(0xDEAD_BEEF));
        }

        #[test]
        fn i64_widened_i32() {
            let v = make_val(TYPE_I32, &[0xFF, 0xFF, 0xFF, 0xFE]);
            assert_eq!(i64::from_smc(v), Some(-2i64));
        }

        #[test]
        fn i64_widened_u16() {
            let v = make_val(TYPE_U16, &[0x01, 0x02]);
            assert_eq!(i64::from_smc(v), Some(0x0102i64));
        }

        #[test]
        fn i64_widened_i8() {
            let v = make_val(TYPE_I8, &[0xFE]);
            assert_eq!(i64::from_smc(v), Some(-2i64));
        }

        #[test]
        fn i64_widened_u8() {
            let v = make_val(TYPE_U8, &[0x2A]);
            assert_eq!(i64::from_smc(v), Some(42i64));
        }

        #[test]
        fn i64_unknown_type_none() {
            let v = make_val(TYPE_FLAG, &[0]);
            assert_eq!(i64::from_smc(v), None);
        }

        #[test]
        fn f32_fpe2() {
            // 25.5 * 4 = 102 = 0x0066
            let v = make_val(TYPE_FPE2, &[0x00, 0x66]);
            assert_eq!(f32::from_smc(v), Some(25.5));
        }

        #[test]
        fn f32_fpe2_zero() {
            let v = make_val(TYPE_FPE2, &[0x00, 0x00]);
            assert_eq!(f32::from_smc(v), Some(0.0));
        }

        #[test]
        fn f32_fpe2_max_unsigned() {
            let v = make_val(TYPE_FPE2, &[0xFF, 0xFF]);
            assert!((f32::from_smc(v).unwrap() - 16383.75).abs() < 0.01);
        }

        #[test]
        fn f32_sp78() {
            // 25.5 * 256 = 6528 = 0x1980
            let v = make_val(TYPE_SP78, &[0x19, 0x80]);
            assert_eq!(f32::from_smc(v), Some(25.5));
        }

        #[test]
        fn f32_sp78_negative() {
            // -1.0 * 256 = -256 = 0xFF00
            let v = make_val(TYPE_SP78, &[0xFF, 0x00]);
            assert_eq!(f32::from_smc(v), Some(-1.0));
        }

        #[test]
        fn f32_flt() {
            // SMC stores flt in native (little-endian) byte order
            // 1.5 as IEEE 754 = 0x3FC00000, LE bytes: [00, 00, C0, 3F]
            let v = make_val(TYPE_FLT, &[0x00, 0x00, 0xC0, 0x3F]);
            assert_eq!(f32::from_smc(v), Some(1.5));
        }

        #[test]
        fn f32_flt_negative() {
            // -1.5 as IEEE 754 = 0xBFC00000, LE bytes: [00, 00, C0, BF]
            let v = make_val(TYPE_FLT, &[0x00, 0x00, 0xC0, 0xBF]);
            assert_eq!(f32::from_smc(v), Some(-1.5));
        }

        #[test]
        fn f32_wrong_type() {
            let v = make_val(TYPE_U16, &[0, 0]);
            assert_eq!(f32::from_smc(v), None);
        }

        #[test]
        fn f32_wrong_size() {
            let v = make_val(TYPE_FLT, &[0, 0, 0]);
            assert_eq!(f32::from_smc(v), None);
        }
    }

    mod umax10 {
        use super::*;

        #[test]
        fn new_valid() {
            for i in 0..=10u8 {
                assert!(UMax10::new(i).is_some());
            }
        }

        #[test]
        fn new_invalid() {
            assert!(UMax10::new(11).is_none());
            assert!(UMax10::new(255).is_none());
        }

        #[test]
        fn new_unchecked_safe_when_contract_met() {
            let u = unsafe { UMax10::new_unchecked(5) };
            assert_eq!(*u, 5);
        }

        #[test]
        fn deref() {
            let u = UMax10::new(7).unwrap();
            assert_eq!(*u, 7);
        }

        #[test]
        fn as_ref() {
            let u = UMax10::new(3).unwrap();
            assert_eq!(u.as_ref(), &3u8);
        }

        #[test]
        fn borrow() {
            use core::borrow::Borrow;
            let u = UMax10::new(9).unwrap();
            assert_eq!(Borrow::<u8>::borrow(&u), &9u8);
        }

        #[test]
        fn display() {
            let u = UMax10::new(10).unwrap();
            assert_eq!(format!("{}", u), "10");
        }

        #[test]
        fn try_from_u8_ok() {
            assert!(UMax10::try_from(5u8).is_ok());
            assert_eq!(UMax10::try_from(10u8).unwrap(), UMax10::new(10).unwrap());
        }

        #[test]
        fn try_from_u8_err() {
            assert!(UMax10::try_from(11u8).is_err());
        }

        #[test]
        fn try_from_u64_ok() {
            assert!(UMax10::try_from(10u64).is_ok());
        }

        #[test]
        fn try_from_u64_err() {
            assert!(UMax10::try_from(11u64).is_err());
        }

        #[test]
        fn from_smc_valid() {
            let v = make_val(TYPE_U8, &[5]);
            assert_eq!(UMax10::from_smc(v).map(|u| *u), Some(5));
        }

        #[test]
        fn from_smc_ten() {
            let v = make_val(TYPE_U8, &[10]);
            assert_eq!(UMax10::from_smc(v).map(|u| *u), Some(10));
        }

        #[test]
        fn from_smc_too_large() {
            let v = make_val(TYPE_U8, &[11]);
            assert_eq!(UMax10::from_smc(v), None);
        }

        #[test]
        fn greater_than_10_impl_display() {
            assert_eq!(format!("{}", GreaterThan10), "number is greater than 10");
        }

        #[test]
        fn debug() {
            let u = UMax10::new(4).unwrap();
            assert_eq!(format!("{:?}", u), "4");
        }
    }

    mod one_digit {
        use super::*;

        #[test]
        fn new_valid() {
            for i in 0..=9u8 {
                assert!(OneDigit::new(i).is_some());
            }
        }

        #[test]
        fn new_invalid() {
            assert!(OneDigit::new(10).is_none());
            assert!(OneDigit::new(255).is_none());
        }

        #[test]
        fn deref() {
            let d = OneDigit::new(9).unwrap();
            assert_eq!(*d, 9);
        }

        #[test]
        fn as_ref() {
            let d = OneDigit::new(0).unwrap();
            assert_eq!(d.as_ref(), &0u8);
        }

        #[test]
        fn try_from_u16_ok() {
            assert!(OneDigit::try_from(9u16).is_ok());
        }

        #[test]
        fn try_from_u16_err() {
            assert!(OneDigit::try_from(10u16).is_err());
        }

        #[test]
        fn try_from_u128_ok() {
            assert!(OneDigit::try_from(0u128).is_ok());
        }

        #[test]
        fn from_smc_valid() {
            let v = make_val(TYPE_U8, &[7]);
            assert_eq!(OneDigit::from_smc(v).map(|d| *d), Some(7));
        }

        #[test]
        fn from_smc_too_large() {
            let v = make_val(TYPE_U8, &[10]);
            assert_eq!(OneDigit::from_smc(v), None);
        }

        #[test]
        fn more_digits_display() {
            assert_eq!(format!("{}", MoreDigits), "number contains more digits");
        }

        #[test]
        fn debug() {
            let d = OneDigit::new(3).unwrap();
            assert_eq!(format!("{:?}", d), "3");
        }
    }
}
