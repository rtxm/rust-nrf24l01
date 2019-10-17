// Copyright 2018, Astro <astro@spaceboyz.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE>. This file
// may not be copied, modified, or distributed except according to
// those terms.

#![no_std]
extern crate embedded_hal;
#[macro_use]
extern crate bitfield;

use core::fmt;
use core::fmt::Debug;
use core::convert::Infallible;
use embedded_hal::blocking::spi::Transfer as SpiTransfer;
use embedded_hal::digital::v2::OutputPin;

mod config;
pub use crate::config::{Configuration, CrcMode, DataRate};
pub mod setup;

mod registers;
use crate::registers::{Config, Register, SetupAw, Status};
mod command;
use crate::command::{Command, ReadRegister, WriteRegister};
mod payload;
pub use crate::payload::Payload;
mod error;
pub use crate::error::Error;

mod device;
pub use crate::device::Device;
mod standby;
pub use crate::standby::StandbyMode;
mod rx;
pub use crate::rx::RxMode;
mod tx;
pub use crate::tx::TxMode;

pub const PIPES_COUNT: usize = 6;
pub const MIN_ADDR_BYTES: usize = 3;
pub const MAX_ADDR_BYTES: usize = 5;

/// Driver for the nRF24L01+
pub struct NRF24L01<CE: OutputPin<Error = Infallible>, CSN: OutputPin<Error = Infallible>, SPI: SpiTransfer<u8>> {
    ce: CE,
    csn: CSN,
    spi: SPI,
    config: Config,
}

impl<CE: OutputPin<Error = Infallible>, CSN: OutputPin<Error = Infallible>, SPI: SpiTransfer<u8, Error = SPIE>, SPIE: Debug> fmt::Debug
    for NRF24L01<CE, CSN, SPI>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NRF24L01")
    }
}

impl<CE: OutputPin<Error = Infallible>, CSN: OutputPin<Error = Infallible>, SPI: SpiTransfer<u8, Error = SPIE>, SPIE: Debug>
    NRF24L01<CE, CSN, SPI>
{
    /// Construct a new driver instance.
    pub fn new(mut ce: CE, mut csn: CSN, spi: SPI) -> Result<StandbyMode<Self>, Error<SPIE>> {
        ce.set_low().unwrap();
        csn.set_high().unwrap();

        // Reset value
        let mut config = Config(0b0000_1000);
        config.set_mask_rx_dr(true);
        config.set_mask_tx_ds(true);
        config.set_mask_max_rt(true);
        let mut device = NRF24L01 {
            ce,
            csn,
            spi,
            config,
        };
        assert!(device.is_connected().unwrap());

        // TODO: activate features?

        StandbyMode::power_up(device).map_err(|(_, e)| e)
    }

    pub fn is_connected(&mut self) -> Result<bool, Error<SPIE>> {
        let (_, setup_aw) = self.read_register::<SetupAw>()?;
        let valid = setup_aw.aw() >= 3 && setup_aw.aw() <= 5;
        Ok(valid)
    }
}

impl<CE: OutputPin<Error = Infallible>, CSN: OutputPin<Error = Infallible>, SPI: SpiTransfer<u8, Error = SPIE>, SPIE: Debug> Device
    for NRF24L01<CE, CSN, SPI>
{
    type Error = Error<SPIE>;

    fn ce_enable(&mut self) {
        self.ce.set_high().unwrap();
    }

    fn ce_disable(&mut self) {
        self.ce.set_low().unwrap();
    }

    fn send_command<C: Command>(
        &mut self,
        command: &C,
    ) -> Result<(Status, C::Response), Self::Error> {
        // Allocate storage
        let mut buf_storage = [0; 33];
        let len = command.len();
        let buf = &mut buf_storage[0..len];
        // Serialize the command
        command.encode(buf);

        // SPI transaction
        self.csn.set_low().unwrap();
        let transfer_result = self.spi.transfer(buf).map(|_| {});
        self.csn.set_high().unwrap();
        // Propagate Err only after csn.set_high():
        transfer_result?;

        // Parse response
        let status = Status(buf[0]);
        let response = C::decode_response(buf);

        Ok((status, response))
    }

    fn write_register<R: Register>(&mut self, register: R) -> Result<Status, Self::Error> {
        let (status, ()) = self.send_command(&WriteRegister::new(register))?;
        Ok(status)
    }

    fn read_register<R: Register>(&mut self) -> Result<(Status, R), Self::Error> {
        self.send_command(&ReadRegister::new())
    }

    fn update_config<F, R>(&mut self, f: F) -> Result<R, Self::Error>
    where
        F: FnOnce(&mut Config) -> R,
    {
        // Mutate
        let old_config = self.config.clone();
        let result = f(&mut self.config);

        if self.config != old_config {
            let config = self.config.clone();
            self.write_register(config)?;
        }
        Ok(result)
    }
}
