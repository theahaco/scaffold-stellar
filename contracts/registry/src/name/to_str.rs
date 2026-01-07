use crate::Error;

pub(crate) trait AsStr {
    fn as_mut_str(&mut self) -> Result<&mut str, Error>;
    fn as_str(&self) -> Result<&str, Error>;
}

impl AsStr for [u8] {
    fn as_mut_str(&mut self) -> Result<&mut str, Error> {
        core::str::from_utf8_mut(self).map_err(|_| Error::InvalidName)
    }

    fn as_str(&self) -> Result<&str, Error> {
        core::str::from_utf8(self).map_err(|_| Error::InvalidName)
    }
}
