use embedded_hal::spi;

/// Setup parameters
pub fn spi_mode() -> spi::Mode {
    spi::Mode {
        polarity: spi::Polarity::IdleLow,
        phase: spi::Phase::CaptureOnFirstTransition,
    }
}

pub fn clock_mhz() -> u32 {
    8
}
