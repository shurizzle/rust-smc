use four_char_code::{four_char_code as fcc, FourCharCode};

use crate::{Result, SMC};

impl SMC {
    fn _keys_len(&self) -> Result<u32> {
        self.read_key::<u32>(fcc!("#KEY"))
    }

    #[inline]
    pub fn keys_len(&self) -> Result<usize> {
        self._keys_len().map(|n| n as usize)
    }

    pub fn keys(&self) -> Result<Keys> {
        let len = self._keys_len()?;
        Ok(Keys {
            smc: self,
            len,
            pos: 0,
        })
    }
}

pub struct Keys<'a> {
    smc: &'a SMC,
    len: u32,
    pos: u32,
}

impl<'a> Iterator for Keys<'a> {
    type Item = Result<FourCharCode>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len > self.pos {
            let i = self.pos;
            self.pos += 1;
            let res = self.smc.get_key(i);
            if res.is_err() {
                self.pos = self.len;
            }
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
        if n > u32::MAX as usize {
            self.pos = self.len;
            return None;
        }

        self.pos = if let Some(pos) = self.pos.checked_add(n as u32) {
            pos.min(self.len)
        } else {
            self.len
        };

        self.next()
    }
}

impl<'a> ExactSizeIterator for Keys<'a> {
    #[inline]
    fn len(&self) -> usize {
        (self.len - self.pos) as usize
    }
}

impl<'a> DoubleEndedIterator for Keys<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.len > self.pos {
            self.len -= 1;
            Some(self.smc.get_key(self.len))
        } else {
            None
        }
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        if n > u32::MAX as usize {
            self.pos = self.len;
            return None;
        }

        self.len = if let Some(len) = self.len.checked_sub(n as u32) {
            len.max(self.pos)
        } else {
            self.pos
        };

        self.next_back()
    }
}
