use core::fmt::Debug;

#[derive(Debug)]
pub enum Error<SPIE: Debug> {
    SpiError(SPIE),
}

impl<SPIE: Debug> From<SPIE> for Error<SPIE> {
    fn from(e: SPIE) -> Self {
        Error::SpiError(e)
    }
}
