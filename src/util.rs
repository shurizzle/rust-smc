use crate::{
    SMCVal, TYPE_FLAG, TYPE_FLT, TYPE_FPE2, TYPE_I16, TYPE_I32, TYPE_I8, TYPE_SP78, TYPE_U16,
    TYPE_U32, TYPE_U8,
};

/// Writes an `f32` into an [`SMCVal`] in the appropriate SMC format
/// (FPE2, SP78, or FLT depending on `val.r#type`).
pub fn write_f32(n: f32, val: &mut SMCVal) -> Option<()> {
    match (val.r#type, val.len()) {
        (TYPE_FPE2, 2) => {
            if n.is_sign_negative() {
                return None;
            }
            unsafe {
                core::ptr::copy_nonoverlapping(
                    ((n * 4.0) as u16).to_be_bytes().as_ptr(),
                    val.data_mut().as_mut_ptr(),
                    2,
                )
            };
            Some(())
        }
        (TYPE_SP78, 2) => {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    ((n * 256.0) as i16).to_be_bytes().as_ptr(),
                    val.data_mut().as_mut_ptr(),
                    2,
                )
            };
            Some(())
        }
        (TYPE_FLT, 4) => {
            // SMC stores flt in native byte order (LE on all Macs)
            unsafe {
                core::ptr::copy_nonoverlapping(
                    n.to_ne_bytes().as_ptr(),
                    val.data_mut().as_mut_ptr(),
                    4,
                )
            };
            Some(())
        }
        _ => None,
    }
}

/// Writes a `u32` into an [`SMCVal`] in U32 format.
pub fn write_u32(n: u32, val: &mut SMCVal) -> Option<()> {
    match (val.r#type, val.len()) {
        (TYPE_U32, 4) => unsafe {
            core::ptr::copy_nonoverlapping(
                n.to_be_bytes().as_ptr(),
                val.data_mut().as_mut_ptr(),
                4,
            );
            Some(())
        },
        _ => None,
    }
}

/// Writes an `i32` into an [`SMCVal`] in I32 format.
pub fn write_i32(n: i32, val: &mut SMCVal) -> Option<()> {
    match (val.r#type, val.len()) {
        (TYPE_I32, 4) => unsafe {
            core::ptr::copy_nonoverlapping(
                n.to_be_bytes().as_ptr(),
                val.data_mut().as_mut_ptr(),
                4,
            );
            Some(())
        },
        _ => None,
    }
}

/// Writes a `u16` into an [`SMCVal`] in U16 format, or widened to I32/U32.
pub fn write_u16(n: u16, val: &mut SMCVal) -> Option<()> {
    match (val.r#type, val.len()) {
        (TYPE_U16, 2) => unsafe {
            core::ptr::copy_nonoverlapping(
                n.to_be_bytes().as_ptr(),
                val.data_mut().as_mut_ptr(),
                2,
            );
            Some(())
        },
        (TYPE_I32, 4) => write_i32(n as i32, val),
        (TYPE_U32, 4) => write_u32(n as u32, val),
        _ => None,
    }
}

/// Writes an `i16` into an [`SMCVal`] in I16 format, or widened to I32.
pub fn write_i16(n: i16, val: &mut SMCVal) -> Option<()> {
    match (val.r#type, val.len()) {
        (TYPE_I16, 2) => unsafe {
            core::ptr::copy_nonoverlapping(
                n.to_be_bytes().as_ptr(),
                val.data_mut().as_mut_ptr(),
                2,
            );
            Some(())
        },
        (TYPE_I32, 4) => write_i32(n as i32, val),
        _ => None,
    }
}

/// Writes a `u8` into an [`SMCVal`] in U8 format, or widened to I16/U16.
pub fn write_u8(n: u8, val: &mut SMCVal) -> Option<()> {
    match (val.r#type, val.len()) {
        (TYPE_U8, 1) => unsafe {
            *val.data_mut().get_unchecked_mut(0) = n;
            Some(())
        },
        (TYPE_I16, 2) => write_i16(n as i16, val),
        _ => write_u16(n as u16, val),
    }
}

/// Writes an `i8` into an [`SMCVal`] in I8 format, or widened to I16.
#[allow(unnecessary_transmutes)]
pub fn write_i8(n: i8, val: &mut SMCVal) -> Option<()> {
    match (val.r#type, val.len()) {
        (TYPE_I8, 1) => unsafe {
            *val.data_mut().get_unchecked_mut(0) = core::mem::transmute::<i8, u8>(n);
            Some(())
        },
        _ => write_i16(n as i16, val),
    }
}

/// Writes a `bool` into an [`SMCVal`] in FLAG format.
pub fn write_bool(n: bool, val: &mut SMCVal) -> Option<()> {
    match (val.r#type, val.len()) {
        (TYPE_FLAG, 1) => unsafe {
            *val.data_mut().get_unchecked_mut(0) = n as u8;
            Some(())
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FromSMC;
    use four_char_code::FourCharCode;

    fn make_val(r#type: FourCharCode, data: &[u8]) -> SMCVal {
        let mut val = SMCVal { r#type, size: data.len(), data: [0; 32] };
        val.data[..data.len()].copy_from_slice(data);
        val
    }

    fn read_val(val: &SMCVal) -> &[u8] {
        &val.data[..val.size]
    }

    mod write_f32 {
        use super::*;

        #[test]
        fn fpe2_normal() {
            let mut v = make_val(TYPE_FPE2, &[0, 0]);
            assert!(write_f32(25.5, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x00, 0x66]);
        }

        #[test]
        fn fpe2_zero() {
            let mut v = make_val(TYPE_FPE2, &[0, 0]);
            assert!(write_f32(0.0, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x00, 0x00]);
        }

        #[test]
        fn fpe2_negative_rejected() {
            let mut v = make_val(TYPE_FPE2, &[0, 0]);
            assert!(write_f32(-1.0, &mut v).is_none());
        }

        #[test]
        fn sp78_normal() {
            let mut v = make_val(TYPE_SP78, &[0, 0]);
            assert!(write_f32(25.5, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x19, 0x80]);
        }

        #[test]
        fn sp78_negative() {
            let mut v = make_val(TYPE_SP78, &[0, 0]);
            assert!(write_f32(-1.0, &mut v).is_some());
            assert_eq!(read_val(&v), &[0xFF, 0x00]);
        }

        #[test]
        fn flt_normal() {
            // SMC stores flt in native byte order (LE on all Macs)
            let mut v = make_val(TYPE_FLT, &[0, 0, 0, 0]);
            assert!(write_f32(1.5, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x00, 0x00, 0xC0, 0x3F]);
        }

        #[test]
        fn flt_negative() {
            let mut v = make_val(TYPE_FLT, &[0, 0, 0, 0]);
            assert!(write_f32(-1.5, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x00, 0x00, 0xC0, 0xBF]);
        }

        #[test]
        fn flt_zero() {
            let mut v = make_val(TYPE_FLT, &[0, 0, 0, 0]);
            assert!(write_f32(0.0, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x00, 0x00, 0x00, 0x00]);
        }

        #[test]
        fn wrong_type() {
            let mut v = make_val(TYPE_U16, &[0, 0]);
            assert!(write_f32(1.0, &mut v).is_none());
        }

        #[test]
        fn wrong_size() {
            let mut v = make_val(TYPE_FPE2, &[0, 0, 0]);
            assert!(write_f32(1.0, &mut v).is_none());
        }
    }

    mod write_u32 {
        use super::*;

        #[test]
        fn normal() {
            let mut v = make_val(TYPE_U32, &[0, 0, 0, 0]);
            assert!(write_u32(0xDEAD_BEEF, &mut v).is_some());
            assert_eq!(read_val(&v), &[0xDE, 0xAD, 0xBE, 0xEF]);
        }

        #[test]
        fn zero() {
            let mut v = make_val(TYPE_U32, &[0, 0, 0, 0]);
            assert!(write_u32(0, &mut v).is_some());
            assert_eq!(read_val(&v), &[0, 0, 0, 0]);
        }

        #[test]
        fn wrong_type() {
            let mut v = make_val(TYPE_U16, &[0, 0]);
            assert!(write_u32(42, &mut v).is_none());
        }
    }

    mod write_i32 {
        use super::*;

        #[test]
        fn positive() {
            let mut v = make_val(TYPE_I32, &[0, 0, 0, 0]);
            assert!(write_i32(42, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x00, 0x00, 0x00, 0x2A]);
        }

        #[test]
        fn negative() {
            let mut v = make_val(TYPE_I32, &[0, 0, 0, 0]);
            assert!(write_i32(-2, &mut v).is_some());
            assert_eq!(read_val(&v), &[0xFF, 0xFF, 0xFF, 0xFE]);
        }
    }

    mod write_u16 {
        use super::*;

        #[test]
        fn direct() {
            let mut v = make_val(TYPE_U16, &[0, 0]);
            assert!(write_u16(0x0102, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x01, 0x02]);
        }

        #[test]
        fn widened_to_u32() {
            let mut v = make_val(TYPE_U32, &[0, 0, 0, 0]);
            assert!(write_u16(0x0102, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x00, 0x00, 0x01, 0x02]);
        }

        #[test]
        fn widened_to_i32() {
            let mut v = make_val(TYPE_I32, &[0, 0, 0, 0]);
            assert!(write_u16(0x0102, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x00, 0x00, 0x01, 0x02]);
        }

        #[test]
        fn wrong_type() {
            let mut v = make_val(TYPE_FLAG, &[0]);
            assert!(write_u16(42, &mut v).is_none());
        }
    }

    mod write_i16 {
        use super::*;

        #[test]
        fn direct_positive() {
            let mut v = make_val(TYPE_I16, &[0, 0]);
            assert!(write_i16(42, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x00, 0x2A]);
        }

        #[test]
        fn direct_negative() {
            let mut v = make_val(TYPE_I16, &[0, 0]);
            assert!(write_i16(-2, &mut v).is_some());
            assert_eq!(read_val(&v), &[0xFF, 0xFE]);
        }

        #[test]
        fn widened_to_i32() {
            let mut v = make_val(TYPE_I32, &[0, 0, 0, 0]);
            assert!(write_i16(-2, &mut v).is_some());
            assert_eq!(read_val(&v), &[0xFF, 0xFF, 0xFF, 0xFE]);
        }
    }

    mod write_u8 {
        use super::*;

        #[test]
        fn direct() {
            let mut v = make_val(TYPE_U8, &[0]);
            assert!(write_u8(42, &mut v).is_some());
            assert_eq!(read_val(&v), &[42]);
        }

        #[test]
        fn widened_to_i16() {
            let mut v = make_val(TYPE_I16, &[0, 0]);
            assert!(write_u8(42, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x00, 0x2A]);
        }

        #[test]
        fn widened_to_u16() {
            let mut v = make_val(TYPE_U16, &[0, 0]);
            assert!(write_u8(42, &mut v).is_some());
            assert_eq!(read_val(&v), &[0x00, 0x2A]);
        }
    }

    mod write_i8 {
        use super::*;

        #[test]
        fn direct_positive() {
            let mut v = make_val(TYPE_I8, &[0]);
            assert!(write_i8(42, &mut v).is_some());
            assert_eq!(read_val(&v), &[42]);
        }

        #[test]
        fn direct_negative() {
            let mut v = make_val(TYPE_I8, &[0]);
            assert!(write_i8(-2, &mut v).is_some());
            assert_eq!(read_val(&v), &[0xFE]);
        }

        #[test]
        fn widened_to_i16() {
            let mut v = make_val(TYPE_I16, &[0, 0]);
            assert!(write_i8(-2, &mut v).is_some());
            assert_eq!(read_val(&v), &[0xFF, 0xFE]);
        }
    }

    mod write_bool {
        use super::*;

        #[test]
        fn true_() {
            let mut v = make_val(TYPE_FLAG, &[0]);
            assert!(write_bool(true, &mut v).is_some());
            assert_eq!(read_val(&v), &[1]);
        }

        #[test]
        fn false_() {
            let mut v = make_val(TYPE_FLAG, &[1]);
            assert!(write_bool(false, &mut v).is_some());
            assert_eq!(read_val(&v), &[0]);
        }

        #[test]
        fn wrong_type() {
            let mut v = make_val(TYPE_U8, &[0]);
            assert!(write_bool(true, &mut v).is_none());
        }
    }

    mod roundtrip {
        use super::*;

        #[test]
        fn fpe2_u16_roundtrip() {
            let mut v = make_val(TYPE_FPE2, &[0, 0]);
            assert!(write_f32(25.5, &mut v).is_some());
            let result = u16::from_be_bytes([v.data[0], v.data[1]]);
            assert_eq!(result, 102);
        }

        #[test]
        fn sp78_i16_roundtrip() {
            let mut v = make_val(TYPE_SP78, &[0, 0]);
            assert!(write_f32(-1.0, &mut v).is_some());
            let result = i16::from_be_bytes([v.data[0], v.data[1]]);
            assert_eq!(result, -256);
        }

        #[test]
        fn flag_roundtrip() {
            let mut v = make_val(TYPE_FLAG, &[0]);
            assert!(write_bool(true, &mut v).is_some());
            let read_back = bool::from_smc(v);
            assert_eq!(read_back, Some(true));
        }
    }
}
