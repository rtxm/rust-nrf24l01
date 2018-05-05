// Copyright 2018, Astro <astro@spaceboyz.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/license/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option.  This file may not be copied, modified, or distributed
// except according to those terms.

#![no_std]
extern crate embedded_hal;
#[macro_use]
extern crate bitfield;
// TODO:
extern crate cortex_m_semihosting;

use core::fmt;
use core::fmt::Debug;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi;
use embedded_hal::blocking::spi::Transfer as SpiTransfer;
use embedded_hal::blocking::delay::DelayUs;

use core::fmt::Write;
use cortex_m_semihosting::hio;

mod config;
pub use config::{Configuration, CrcMode, DataRate};
pub mod setup;

mod registers;
use registers::{Register, Config, Status, SetupAw};
mod command;
use command::{Command, ReadRegister, WriteRegister, Nop};
mod payload;
pub use payload::Payload;
mod error;
pub use error::Error;

mod device;
use device::Device;
mod standby;
pub use standby::StandbyMode;
mod rx;
pub use rx::RxMode;
mod tx;
pub use tx::TxMode;

pub const PIPES_COUNT: usize = 6;


/// Driver for the nRF24L01+
// TODO: interrupt resets
pub struct NRF24L01<CE: OutputPin, CSN: OutputPin, SPI: SpiTransfer<u8>, D: DelayUs<u16>> {
    ce: CE,
    csn: CSN,
    spi: SPI,
    delay: D,
    config: Config,
}

impl<CE: OutputPin, CSN: OutputPin, SPI: SpiTransfer<u8, Error=SPIE>, SPIE: Debug, D: DelayUs<u16>> fmt::Debug for NRF24L01<CE, CSN, SPI, D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NRF24L01")
    }
}

impl<CE: OutputPin, CSN: OutputPin, SPI: SpiTransfer<u8, Error=SPIE>, SPIE: Debug, D: DelayUs<u16>> NRF24L01<CE, CSN, SPI, D> {
    /// Construct a new driver instance.
    pub fn new(mut ce: CE, mut csn: CSN, spi: SPI, delay: D) -> Result<StandbyMode<Self>, Error<SPIE>> {
        ce.set_low();
        csn.set_high();

        // Reset value
        let mut config = Config(0b0000_1000);
        config.set_mask_rx_dr(true);
        config.set_mask_tx_ds(true);
        config.set_mask_max_rt(true);
        let mut device = NRF24L01 {
            ce, csn, spi, delay,
            config,
        };
        assert!(device.is_connected().unwrap());

        // TODO: activate features?
        
        StandbyMode::power_up(device)
            .map_err(|(_, e)| e)
    }

    pub fn is_connected(&mut self) -> Result<bool, Error<SPIE>> {
        let (_, setup_aw) =
            self.read_register::<SetupAw>()?;
        let valid =
            setup_aw.aw() >= 3 &&
            setup_aw.aw() <= 5;
        Ok(valid)
    }
}

impl<CE: OutputPin, CSN: OutputPin, SPI: SpiTransfer<u8, Error=SPIE>, SPIE: Debug, D: DelayUs<u16>> Device for NRF24L01<CE, CSN, SPI, D> {
    type Error = Error<SPIE>;

    fn delay_us(&mut self, delay: u16) {
        self.delay.delay_us(delay);
    }
    
    fn ce_enable(&mut self) {
        self.ce.set_high();
    }

    fn ce_disable(&mut self) {
        self.ce.set_low();
    }

    fn send_command<C: Command>(&mut self, command: &C) -> Result<(Status, C::Response), Self::Error> {
        // Allocate storage
        let mut buf_storage = [0; 33];
        let len = command.len();
        let buf = &mut buf_storage[0..len];
        // Serialize the command
        command.encode(buf);

        // let mut stdout = hio::hstdout().unwrap();
        // for b in buf.iter() {
        //     write!(stdout, "{:02X} ", b);
        // }
        // write!(stdout, ">>");

        // SPI transaction
        self.csn.set_low();
        self.spi.transfer(buf)?;
        self.csn.set_high();

        // for b in buf.iter() {
        //     write!(stdout, " {:02X}", b);
        // }
        // write!(stdout, "\n");
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
        where F: FnOnce(&mut Config) -> R
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
