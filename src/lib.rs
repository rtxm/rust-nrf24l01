// Copyright 2017, Romuald Texier-Marcadé <romualdtm@gmail.com>
//           2018, Astro <astro@spaceboyz.net>
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
use command::{Command, ReadRegister, WriteRegister};
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
    ///
    /// # Errors
    ///
    /// System IO errors
    ///
    pub fn new(mut ce: CE, mut csn: CSN, spi: SPI, delay: D) -> Result<StandbyMode<Self>, Error<SPIE>> {
        ce.set_low();
        csn.set_high();

        let mut device = NRF24L01 {
            ce, csn, spi, delay,
            // Reset value
            config: Config(0b0000_1000),
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

    fn ce_enable(&mut self) {
        let mut stdout = hio::hstdout().unwrap();
        writeln!(stdout, "CE high");
        self.ce.set_high();
    }

    fn ce_disable(&mut self) {
        let mut stdout = hio::hstdout().unwrap();
        writeln!(stdout, "CE low");
        self.ce.set_low();
    }

    fn send_command<C: Command>(&mut self, command: &C) -> Result<(Status, C::Response), Self::Error> {
        // Allocate storage
        let mut buf_storage = [0; 33];
        let len = command.len();
        let buf = &mut buf_storage[0..len];
        // Serialize the command
        command.encode(buf);

        let mut stdout = hio::hstdout().unwrap();
        for b in buf.iter() {
            write!(stdout, "{:02X} ", b);
        }
        write!(stdout, ">>");

        // SPI transaction
        self.csn.set_low();
        //self.delay.delay_us(2);
        self.spi.transfer(buf)?;
        //self.delay.delay_us(50);
        self.csn.set_high();
        //self.delay.delay_us(50);

        for b in buf.iter() {
            write!(stdout, " {:02X}", b);
        }
        write!(stdout, "\n");
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

    fn update_config<F>(&mut self, f: F) -> Result<(), Self::Error>
        where F: FnOnce(&mut Config)
    {
        // Mutate
        let old_config = self.config.clone();
        f(&mut self.config);

        if self.config != old_config {
            let config = self.config.clone();
            let mut stdout = hio::hstdout().unwrap();
            writeln!(stdout, "config={:08b}", config.0);
            self.write_register(config)?;
        }
        Ok(())
    }

    
    // fn setup_rf(&mut self, rate: DataRate, level: PALevel) -> Result<(), Error<SPIE>> {
    //     let rate_bits: u8 = match rate {
    //         DataRate::R250Kbps => 0b0010_0000,
    //         DataRate::R1Mbps => 0,
    //         DataRate::R2Mbps => 0b0000_1000,
    //     };
    //     let level_bits: u8 = match level {
    //         PALevel::Min => 0,
    //         PALevel::Low => 0b0000_0010,
    //         PALevel::High => 0b0000_0100,
    //         PALevel::Max => 0b0000_0110,
    //     };
    //     self.write_register(RF_SETUP, rate_bits | level_bits)
    // }

    // fn set_channel(&mut self, channel: u8) -> Result<(), Error<SPIE>> {
    //     if channel < 126 {
    //         self.write_register(RF_CH, channel)
    //     } else {
    //         self.write_register(RF_CH, 125)
    //     }
    // }

    // fn set_full_address(&mut self, pipe: Register, address: &[u8]) -> Result<(), Error<SPIE>> {
    //     let mut response_buffer = [0u8; 6];
    //     let mut command = [W_REGISTER | pipe, 0, 0, 0, 0, 0];
    //     let len = 1 + address.len();
    //     command[1..len].copy_from_slice(&address);
    //     self.send_command(&command[0..len], &mut response_buffer[0..len])
    // }

    // fn set_auto_ack(&mut self, pipes_auto_ack: [bool; 6]) -> Result<(), Error<SPIE>> {
    //     let mut register = 0;
    //     for (i, auto_ack) in pipes_auto_ack.iter().enumerate() {
    //         if *auto_ack {
    //             register |= 1 << i;
    //         }
    //     }
    //     // auto acknowlegement
    //     self.write_register(EN_AA, register)
    // }

    // fn configure_receiver(&mut self, config: &RXConfig) -> Result<u8, Error<SPIE>> {
    //     // set data rate
    //     // set PA level
    //     self.setup_rf(config.data_rate, config.pa_level)?;
    //     // set channel
    //     self.set_channel(config.channel)?;
    //     // set address width
    //     let aw = config.address_width;
    //     assert!(aw >= 3 && aw <= 5);
    //     self.write_register(SETUP_AW, aw as u8 - 2)?;
    //     // set Pipe 0 address
    //     self.set_full_address(RX_ADDR_P0, &config.pipe0_address[0..aw])?;
    //     let mut enabled = 1u8;
    //     // Pipe 1
    //     if let Some(address) = config.pipe1_address {
    //         self.set_full_address(RX_ADDR_P1, &address[0..aw])?;
    //         enabled |= 0b0000_0010
    //     };
    //     // Pipe 2
    //     if let Some(lsb) = config.pipe2_addr_lsb {
    //         self.write_register(RX_ADDR_P2, lsb)?;
    //         enabled |= 0b0000_0100
    //     };
    //     // Pipe 3
    //     if let Some(lsb) = config.pipe3_addr_lsb {
    //         self.write_register(RX_ADDR_P3, lsb)?;
    //         enabled |= 0b0000_1000
    //     }
    //     // Pipe 4
    //     if let Some(lsb) = config.pipe4_addr_lsb {
    //         self.write_register(RX_ADDR_P4, lsb)?;
    //         enabled |= 0b0001_0000
    //     };
    //     // Pipe 5
    //     if let Some(lsb) = config.pipe5_addr_lsb {
    //         self.write_register(RX_ADDR_P5, lsb)?;
    //         enabled |= 0b0010_0000
    //     };
    //     // Configure dynamic payload lengths
    //     let mut feature = 0;
    //     if config.pipes_static_packet_len.iter().any(|pipe_packet_len| pipe_packet_len.is_some()) {
    //         feature |= FEATURE_EN_DPL;
    //     }
    //     if config.pipes_auto_ack.iter().any(|pipe_auto_ack| *pipe_auto_ack) {
    //         feature |= FEATURE_EN_ACK_PAY;
    //     }
    //     self.write_register(FEATURE, feature)?;
    //     // Configure static payload lengths
    //     let mut dynpd = 0;
    //     for (i, c) in config.pipes_static_packet_len.iter().enumerate() {
    //         match *c {
    //             Some(len) => {
    //                 assert!(len < (1 << 6));
    //                 self.write_register(RX_PW[i], len)?;
    //             }
    //             None => {
    //                 // Enable dynamic payload length
    //                 dynpd |= 1 << i;
    //             }
    //         }
    //     }
    //     self.write_register(DYNPD, dynpd)?;
    //     // Configure Auto-ack
    //     self.set_auto_ack(config.pipes_auto_ack)?;
    //     // Enable configured pipes
    //     self.write_register(EN_RXADDR, enabled)?;
    //     // base config is 2 bytes for CRC and RX mode on
    //     // only reflect RX_DR on the IRQ pin
    //     let mut base_config = 0b0111_0000 | CONFIG_PRIM_RX;
    //     config.crc_mode.map(|crc_mode| {
    //         base_config |= crc_mode.config_mask();
    //     });

    //     Ok(base_config)
    // }

    // fn configure_transmitter(&mut self, config: &TXConfig) -> Result<u8, Error<SPIE>> {
    //     // set data rate
    //     // set PA level
    //     self.setup_rf(config.data_rate, config.pa_level)?;
    //     // set channel
    //     self.set_channel(config.channel)?;
    //     // set address width
    //     let aw = config.address_width;
    //     assert!(aw >= 3 && aw <= 5);
    //     self.write_register(SETUP_AW, aw as u8 - 2)?;
    //     // set destination and Pipe 0 address
    //     self.set_full_address(RX_ADDR_P0, &config.pipe0_address[0..aw])?;
    //     self.set_full_address(TX_ADDR, &config.pipe0_address[0..aw])?;
    //     // disable other pipes
    //     self.write_register(EN_RXADDR, 1u8)?;
    //     // retransmission settings
    //     let retry_bits: u8 = config.max_retries.min(15);
    //     let retry_delay_bits: u8 = config.retry_delay.min(0xF);
    //     self.write_register(
    //         SETUP_RETR,
    //         (retry_delay_bits << 4) | retry_bits,
    //     )?;
    //     // base config is 2 bytes for CRC and TX mode on
    //     // only reflect TX_DS and MAX_RT on the IRQ pin
    //     let mut base_config = 0b0111_0000;
    //     config.crc_mode.map(|crc_mode| {
    //         base_config |= crc_mode.config_mask();
    //     });

    //     Ok(base_config)
    // }

    // Public API


    // /// Configure the device as Primary Receiver (PRX) or Primary Transmitter (PTX),
    // /// set all its properties for proper operation and power it up.
    // ///
    // /// The device remain in standby until `self.listen()` (RX mode)
    // /// or `self.send()` (TX mode) is called.
    // ///
    // /// All commands work when the device is in standby (recommended) as well as
    // /// active state.
    // pub fn configure(&mut self, mode: &OperatingMode) -> Result<(), Error<SPIE>> {
    //     self.ce.set_low();

    //     // Mode specific configuration
    //     match *mode {
    //         OperatingMode::RX(ref config) => self.configure_receiver(config),
    //         OperatingMode::TX(ref config) => self.configure_transmitter(config),
    //     }.and_then(|base_config| {
    //         // Reset status
    //         self.write_register(STATUS, STATUS_RX_DR | STATUS_TX_DS | STATUS_MAX_RT);
    //         // Go!
    //         self.base_config = base_config;
    //         self.power_up()?;
    //         self.delay.delay_us(150);
    //         Ok(())
    //     })
    // }
}
