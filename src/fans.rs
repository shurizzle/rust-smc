use core::{borrow::Borrow, fmt, mem::MaybeUninit, ops::Deref};

use four_char_code::{fcc_format, four_char_code, FourCharCode};

use crate::{util, FromSMC, IntoSMC, OneDigit, Result, SMCError, UMax10, SMC};

const TYPE_FAN: FourCharCode = four_char_code!("{fds");

struct FanName([u8; 32 - 4], usize);

impl FromSMC for FanName {
    fn from_smc(param: crate::SMCVal) -> Option<Self> {
        if param.r#type != TYPE_FAN || param.len() < 4 {
            return None;
        }

        unsafe {
            let b = param.data().get_unchecked(4..);
            let mut buf = MaybeUninit::<[u8; 32 - 4]>::uninit();
            let mut ptr = buf.as_mut_ptr().cast::<u8>();

            for c in b {
                if *c == 0 {
                    break;
                } else {
                    *ptr = *c;
                    ptr = ptr.add(1);
                }
            }
            let len = (ptr as usize) - (buf.as_mut_ptr().cast::<u8>() as usize);

            Some(FanName(buf.assume_init(), len))
        }
    }
}

impl Deref for FanName {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.get_unchecked(..self.1) }
    }
}

impl AsRef<[u8]> for FanName {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl Borrow<[u8]> for FanName {
    fn borrow(&self) -> &[u8] {
        self
    }
}

fn is_managed(fan: &Fan, smc: &SMC) -> Result<bool> {
    Ok(smc.managed_fans()? & (1u16 << (*fan.id as u16)) == 0)
}

fn rpm(current: f32, min: f32) -> f32 {
    let mut rpm = current - min;
    if rpm < 0.0 {
        rpm = 0.0;
    }
    rpm
}

pub fn percent(current: f32, min: f32, max: f32) -> f32 {
    let rpm = current - min;
    let rpm = if rpm < 0.0 { 0.0 } else { rpm };

    rpm / (max - min) * 100.0
}

// hide safe initialization
mod __private {
    use crate::{util, IntoSMC};

    pub struct CheckedSpeed(f32);

    impl CheckedSpeed {
        #[inline]
        pub unsafe fn new_unchecked(value: f32) -> Self {
            Self(value)
        }
    }

    impl IntoSMC for CheckedSpeed {
        #[inline]
        fn into_smc(self, param: &mut crate::SMCVal) -> Option<()> {
            util::write_f32(self.0, param)
        }
    }
}
use __private::CheckedSpeed;

pub fn set_min_speed(smc: &mut SMC, id: OneDigit, max: f32, speed: f32) -> Result<()> {
    if speed <= 0.0 || speed > max {
        Err(SMCError::TryInto)
    } else {
        unsafe {
            smc.write_key(
                fcc_format!("F{}Mn", *id)?,
                CheckedSpeed::new_unchecked(speed),
            )
        }
    }
}

pub fn set_current_speed(
    smc: &mut SMC,
    id: OneDigit,
    min: f32,
    max: f32,
    speed: f32,
) -> Result<()> {
    if speed < min || speed > max {
        Err(SMCError::TryInto)
    } else {
        unsafe {
            smc.write_key(
                fcc_format!("F{}Tg", *id)?,
                CheckedSpeed::new_unchecked(speed),
            )
        }
    }
}

pub struct Fan {
    id: OneDigit,
    name: FanName,
}

impl Fan {
    #[inline]
    pub fn id(&self) -> OneDigit {
        self.id
    }

    #[inline]
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    #[inline]
    pub fn min_speed(&self, smc: &SMC) -> Result<f32> {
        smc.get_fan_min_speed(self.id())
    }

    #[inline]
    pub fn max_speed(&self, smc: &SMC) -> Result<f32> {
        smc.get_fan_max_speed(self.id())
    }

    #[inline]
    pub fn current_speed(&self, smc: &SMC) -> Result<f32> {
        smc.get_fan_current_speed(self.id())
    }

    #[inline]
    pub fn is_managed(&self, smc: &SMC) -> Result<bool> {
        is_managed(self, smc)
    }

    pub fn rpm(&self, smc: &SMC) -> Result<f32> {
        Ok(rpm(self.current_speed(smc)?, self.min_speed(smc)?))
    }

    pub fn set_managed(&self, smc: &mut SMC, managed: bool) -> Result<()> {
        smc.fan_set_managed(self.id(), managed)
    }

    pub fn percent(&self, smc: &SMC) -> Result<f32> {
        Ok(percent(
            self.current_speed(smc)?,
            self.min_speed(smc)?,
            self.max_speed(smc)?,
        ))
    }

    pub fn set_min_speed(&self, smc: &mut SMC, speed: f32) -> Result<()> {
        let max = self.max_speed(smc)?;
        set_min_speed(smc, self.id(), max, speed)
    }

    pub fn set_current_speed(&self, smc: &mut SMC, speed: f32) -> Result<()> {
        let min = self.min_speed(smc)?;
        let max = self.max_speed(smc)?;
        set_current_speed(smc, self.id(), min, max, speed)
    }

    pub fn into_info(self, smc: &SMC) -> Result<FanInfo> {
        let min_speed = self.min_speed(smc)?;
        let max_speed = self.max_speed(smc)?;
        let current_speed = self.current_speed(smc)?;
        let managed = self.is_managed(smc)?;

        Ok(FanInfo {
            fan: self,
            min_speed,
            max_speed,
            current_speed,
            managed,
        })
    }
}

pub struct FanInfo {
    fan: Fan,
    min_speed: f32,
    max_speed: f32,
    current_speed: f32,
    managed: bool,
}

impl FanInfo {
    #[inline]
    pub fn id(&self) -> OneDigit {
        self.fan.id()
    }

    #[inline]
    pub fn name(&self) -> &[u8] {
        self.fan.name()
    }

    #[inline]
    pub fn min_speed(&self) -> f32 {
        self.min_speed
    }

    #[inline]
    pub fn max_speed(&self) -> f32 {
        self.max_speed
    }

    #[inline]
    pub fn current_speed(&self) -> f32 {
        self.current_speed
    }

    #[inline]
    pub fn is_managed(&self) -> bool {
        self.managed
    }

    #[inline]
    pub fn rpm(&self) -> f32 {
        rpm(self.current_speed(), self.min_speed())
    }

    #[inline]
    pub fn percent(&self) -> f32 {
        percent(self.current_speed(), self.min_speed(), self.max_speed())
    }

    #[inline]
    pub fn set_managed(&self, smc: &mut SMC, managed: bool) -> Result<()> {
        smc.fan_set_managed(self.id(), managed)
    }

    #[inline]
    pub fn set_min_speed(&self, smc: &mut SMC, speed: f32) -> Result<()> {
        set_min_speed(smc, self.id(), self.max_speed(), speed)
    }

    #[inline]
    pub fn set_current_speed(&self, smc: &mut SMC, speed: f32) -> Result<()> {
        set_current_speed(smc, self.id(), self.min_speed(), self.max_speed(), speed)
    }

    #[inline]
    pub fn into_fan(self) -> Fan {
        self.fan
    }

    pub fn refresh(&mut self, smc: &SMC) -> Result<()> {
        self.min_speed = smc.get_fan_min_speed(self.id())?;
        self.max_speed = smc.get_fan_max_speed(self.id())?;
        self.current_speed = smc.get_fan_current_speed(self.id())?;
        self.managed = smc.managed_fans()? & (1u16 << (*self.fan.id as u16)) == 0;

        Ok(())
    }
}

impl From<FanInfo> for Fan {
    #[inline]
    fn from(value: FanInfo) -> Self {
        value.into_fan()
    }
}

impl fmt::Debug for FanInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FanInfo")
            .field("id", &self.id())
            .field("name", &self.name())
            .field("min_speed", &self.min_speed)
            .field("max_speed", &self.max_speed)
            .field("current_speed", &self.current_speed)
            .finish()
    }
}

impl fmt::Debug for Fan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fan")
            .field("id", &self.id)
            .field("name", &self.name())
            .finish()
    }
}

impl SMC {
    #[inline]
    fn _fans_len(&self) -> Result<UMax10> {
        self.read_key::<UMax10>(four_char_code!("FNum"))
    }

    pub fn fans_len(&self) -> Result<usize> {
        self._fans_len().map(|n| *n as usize)
    }

    pub fn get_fan(&self, id: OneDigit) -> Result<Fan> {
        let key = fcc_format!("F{}ID", *id)?;
        self.read_key::<FanName>(key).map(|name| Fan { id, name })
    }

    pub fn get_fan_min_speed(&self, id: OneDigit) -> Result<f32> {
        self.read_key(fcc_format!("F{}Mn", *id)?)
    }

    pub fn get_fan_max_speed(&self, id: OneDigit) -> Result<f32> {
        self.read_key(fcc_format!("F{}Mx", *id)?)
    }

    pub fn get_fan_current_speed(&self, id: OneDigit) -> Result<f32> {
        self.read_key(fcc_format!("F{}Ac", *id)?)
    }

    pub fn get_fan_info(&self, id: OneDigit) -> Result<FanInfo> {
        self.get_fan(id)?.into_info(self)
    }

    pub fn fans(&self) -> Result<Fans> {
        let len = *self._fans_len()?;
        Ok(Fans {
            smc: self,
            pos: 0,
            len,
        })
    }

    pub fn fan_infos(&self) -> Result<FanInfos> {
        Ok(FanInfos {
            inner: self.fans()?,
        })
    }

    #[inline]
    pub fn managed_fans(&self) -> Result<u16> {
        self.read_key(four_char_code!("FS! "))
    }

    pub fn fan_set_managed(&mut self, id: OneDigit, managed: bool) -> Result<()> {
        let bitmask = self.managed_fans()?;
        let mask = 1u16 << (*id as u16);
        let new: u16 = if managed {
            bitmask & !mask
        } else {
            bitmask | mask
        };

        struct CheckedBitmask(u16);
        impl IntoSMC for CheckedBitmask {
            fn into_smc(self, param: &mut crate::SMCVal) -> Option<()> {
                util::write_u16(self.0, param)
            }
        }

        if bitmask != new {
            unsafe { self.write_key(four_char_code!("FS! "), CheckedBitmask(new)) }
        } else {
            Ok(())
        }
    }
}

pub struct Fans<'a> {
    smc: &'a SMC,
    pos: u8,
    len: u8,
}

impl<'a> Fans<'a> {
    #[inline]
    pub fn smc(&self) -> &'a SMC {
        self.smc
    }
}

impl<'a> Iterator for Fans<'a> {
    type Item = Result<Fan>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len > self.pos {
            let res = self
                .smc
                .get_fan(unsafe { OneDigit::new_unchecked(self.pos) });
            self.pos = if res.is_ok() { self.pos + 1 } else { self.len };
            Some(res)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.len()
    }

    fn last(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        if self.len > self.pos {
            self.len = self.pos - 1;
        } else {
            return None;
        }
        self.next()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if n > u8::MAX as usize {
            self.pos = self.len;
            return None;
        }

        self.pos = if let Some(pos) = self.pos.checked_add(n as u8) {
            pos.min(self.len)
        } else {
            self.len
        };

        self.next()
    }
}

impl<'a> ExactSizeIterator for Fans<'a> {
    #[inline]
    fn len(&self) -> usize {
        (self.len - self.pos) as usize
    }
}

impl<'a> DoubleEndedIterator for Fans<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len > self.pos {
            let res = self
                .smc
                .get_fan(unsafe { OneDigit::new_unchecked(self.pos) });
            self.len = if res.is_ok() { self.len - 1 } else { self.pos };
            Some(res)
        } else {
            None
        }
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        if n > u8::MAX as usize {
            self.pos = self.len;
            return None;
        }

        self.len = if let Some(len) = self.len.checked_sub(n as u8) {
            len.max(self.pos)
        } else {
            self.pos
        };

        self.next_back()
    }
}

pub struct FanInfos<'a> {
    inner: Fans<'a>,
}

impl<'a> Iterator for FanInfos<'a> {
    type Item = Result<FanInfo>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|o| o.and_then(|fan| fan.into_info(self.inner.smc())))
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        let smc = self.inner.smc();
        self.inner
            .last()
            .map(|o| o.and_then(|fan| fan.into_info(smc)))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }

    #[inline]
    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.inner.count()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.inner
            .nth(n)
            .map(|o| o.and_then(|fan| fan.into_info(self.inner.smc())))
    }
}

impl<'a> ExactSizeIterator for FanInfos<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'a> DoubleEndedIterator for FanInfos<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|o| o.and_then(|fan| fan.into_info(self.inner.smc())))
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.inner
            .nth_back(n)
            .map(|o| o.and_then(|fan| fan.into_info(self.inner.smc())))
    }
}
