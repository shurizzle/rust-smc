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
            unsafe {
                core::ptr::copy_nonoverlapping(
                    n.to_be_bytes().as_ptr(),
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
