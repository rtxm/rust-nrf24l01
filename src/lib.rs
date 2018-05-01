// Copyright 2017, Romuald Texier-Marcadé <romualdtm@gmail.com>
//           2018, Astro <astro@spaceboyz.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/license/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option.  This file may not be copied, modified, or distributed
// except according to those terms.

#![no_std]

#[macro_use(block)]
extern crate nb;
extern crate embedded_hal;
// TODO:
extern crate cortex_m_semihosting;

use core::fmt::Debug;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi;
use embedded_hal::blocking::spi::Transfer as SpiTransfer;
use embedded_hal::blocking::delay::DelayUs;

use core::fmt::Write;
use cortex_m_semihosting::hio;

/// Supported air data rates.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum DataRate {
    R250Kbps,
    R1Mbps,
    R2Mbps,
}

impl Default for DataRate {
    fn default() -> DataRate {
        DataRate::R1Mbps
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CrcMode {
    OneByte,
    TwoBytes,
}

impl CrcMode {
    fn config_mask(&self) -> u8 {
        match *self {
            CrcMode::OneByte => CONFIG_EN_CRC,
            CrcMode::TwoBytes => CONFIG_EN_CRC | CONFIG_CRC0,
        }
    }
}

/// Supported power amplifier levels.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum PALevel {
    /// -18 dBm, 7.0 mA DC current consumption, few meters range.
    Min,
    /// -12 dBM, 7.5 mA DC current consumption.
    Low,
    /// -6 dBm, 9.0 mA DC current consumption.
    High,
    /// 0 dBm, 11.3 mA DC current consumption, up to 100 meters range.
    Max,
}

impl Default for PALevel {
    fn default() -> PALevel {
        PALevel::Min
    }
}

/// Receiver mode configuration
#[derive(Debug)]
#[derive(Clone)]
#[derive(Default)]
pub struct RXConfig {
    /// data rate, defaults to `DataRate::R1Mbps`.
    pub data_rate: DataRate,
    /// channel, in the range [0, 125], defaults to 0.
    ///
    /// The RF channel frequency F is set according to the formula:
    ///
    /// F = 2400 + `channel` Mhz
    ///
    /// Any `channel` value above 125 is capped to 125.
    pub channel: u8,
    /// Powel level, defaults to `PALevel::Min`.
    pub pa_level: PALevel,

    pub address_width: usize,
    /// Pipe 0 address
    ///
    /// This is the receiving base address.
    ///
    /// Typically, this is the only address you need to set, unless you
    /// need a multiceiver configuration. In that case, you can enable up to
    /// five additional receiving pipes.
    ///
    /// The address is in little endian order: the first byte is the least significant one.
    ///
    /// You must provide a valid address for Pipe 0.
    pub pipe0_address: [u8; 5],
    /// Pipe 1 address, defaults to None (disabled)
    ///
    /// All pipes 2-5 share the 4 most significant bytes with the pipe 1 address, so
    /// you only need to provide the least significant byte to enable one of those pipes or
    /// set it to None to disable it (default).
    pub pipe1_address: Option<[u8; 5]>,
    /// Pipe 2 LSB, defaults to None (disabled)
    pub pipe2_addr_lsb: Option<u8>,
    /// Pipe 3 LSB, defaults to None (disabled)
    pub pipe3_addr_lsb: Option<u8>,
    /// Pipe 4 LSB, defaults to None (disabled)
    pub pipe4_addr_lsb: Option<u8>,
    /// Pipe 5 LSB, defaults to None (disabled)
    pub pipe5_addr_lsb: Option<u8>,

    pub pipes_static_packet_len: [Option<u8>; 6],

    /// Per-pipe enable auto-acknowledgement
    pub pipes_auto_ack: [bool; 6],

    /// CRC
    pub crc_mode: Option<CrcMode>,
}


/// Transmitter mode configuration
#[derive(Debug)]
#[derive(Clone)]
#[derive(Default)]
pub struct TXConfig {
    /// data rate, defaults to `DataRate::R1Mbps`
    ///
    /// Both Transmitter and Receiver ends should use the same data rate.
    pub data_rate: DataRate,
    /// channel, in the range [0, 125], defaults to 0.
    ///
    /// The RF channel frequency F is set according to the formula:
    ///
    /// F = 2400 + `channel` Mhz
    ///
    /// Any `channel` value above 125 is capped to 125.
    ///
    /// Both Transmitter and Receiver ends should use the same channel.
    pub channel: u8,
    /// Powel level, defaults to `PALevel::Min`.
    pub pa_level: PALevel,
    /// Max number of retries before giving up when trying to send a packet.
    ///
    /// 0 <= `max_retries` <= 15. Default is 0. Any value above 15 is capped to 15.
    pub max_retries: u8, // [0, 15]
    /// Delay (in steps of 250µs) between retries.
    ///
    /// Actual delay = 250 + `retry_delay` * 250 [µs]
    ///
    /// 0 <= `retry_delay` <= 15. Default is 0, recommended is > 1.
    /// Any value above 15 is capped to 15.
    pub retry_delay: u8, // [0, 15]

    pub address_width: usize,

    /// Destination address, should match an address on the receiver end.
    ///
    /// This is also the address on which ACK packets are received.
    /// The address is in little endian order: the first byte is the least significant one.
    pub pipe0_address: [u8; 5],

    /// CRC
    pub crc_mode: Option<CrcMode>,
}


/// The Operating mode, either Receiver or Transmitter.
#[derive(Debug)]
pub enum OperatingMode {
    /// Primary receiver
    RX(RXConfig),
    /// Primary transmitter
    TX(TXConfig),
}

type Command = u8;

// Read register
const R_REGISTER: Command = 0;
// Write register
const W_REGISTER: Command = 0b0010_0000;
// Read input FIFO
const R_RX_PAYLOAD: Command = 0b0110_0001;
// Read the size of the packet on top of input FIFO
const R_RX_PL_WID: Command = 0b0110_0000;
// Push a packet to output FIFO
const W_TX_PAYLOAD: Command = 0b1010_0000;
// Push an ACK packet to output FIFO
// Use the last three bits to specify the pipe number
const W_ACK_PAYLOAD: Command = 0b1010_1000;
// Flush commands
const FLUSH_TX: Command = 0b1110_0001;
const FLUSH_RX: Command = 0b1110_0010;

type Register = u8;

// Base config, p 54
const CONFIG: Register = 0;
const CONFIG_EN_CRC: u8 = 1 << 3;
const CONFIG_CRC0: u8 = 1 << 2;
const CONFIG_PWR_UP: u8 = 1 << 1;
const CONFIG_PRIM_RX: u8 = 1;
// Enable auto acknowlegment, p54
const EN_AA: Register = 0x01;
// Enabled RX addresses, p 54
const EN_RXADDR: Register = 0x02;
const SETUP_AW: Register = 0x03;
// Setup of automatic retransmission, p 55
const SETUP_RETR: Register = 0x04;
// Channel, p 55
const RF_CH: Register = 0x05;
// RF data rate and power, p 55
const RF_SETUP: Register = 0x06;
// The status register is returned for each command, so we don't need
// to reed it explicitly.
// We may need to write to it to clear some flags (RX_DR, TX_DS, MAX_RT)
// p 56
const STATUS: Register = 0x07;
const STATUS_RX_DR: u8 = 1 << 6;
const STATUS_TX_DS: u8 = 1 << 5;
const STATUS_MAX_RT: u8 = 1 << 4;
const STATUS_RX_P_NO: u8 = 0b111 << 1;
// Transmission quality, p 56
const OBSERVE_TX: Register = 0x08;
// Pipe 0 address, p 57
const RX_ADDR_P0: Register = 0x0A;
// Pipe 1 address, p 57
const RX_ADDR_P1: Register = 0x0B;
// Pipe 2 address, p 57
const RX_ADDR_P2: Register = 0x0C;
// Pipe 3 address, p 57
const RX_ADDR_P3: Register = 0x0D;
// Pipe 4 address, p 57
const RX_ADDR_P4: Register = 0x0E;
// Pipe 5 address, p 57
const RX_ADDR_P5: Register = 0x0F;
// Number of bytes in RX payload for each of the pipes
const RX_PW: [Register; 6] = [0x11, 0x12, 0x13, 0x14, 0x15, 0x16];
// Destination address, p 57
const TX_ADDR: Register = 0x10;
// FIFO status (RX & TX), p 58
const FIFO_STATUS: Register = 0x17;
// TX FIFO empty flag
const FIFO_STATUS_TX_FULL: u8 = 1 << 5;
const FIFO_STATUS_TX_EMPTY: u8 = 1 << 4;
const FIFO_STATUS_RX_FULL: u8 = 1 << 1;
const FIFO_STATUS_RX_EMPTY: u8 = 1 << 0;
// Enable dynamic payload length (requires EN_DPL and ENAA_PX), p 59
const DYNPD: Register = 0x1C;
//  Feature register (content EN_DPL, EN_ACK_PAY...), p 59
const FEATURE: Register = 0x1D;
const FEATURE_EN_DPL: u8 = 1 << 2;
const FEATURE_EN_ACK_PAY: u8 = 1 << 1;


#[derive(Debug)]
pub enum Error<SPIE: Debug> {
    SpiError(SPIE),
}

impl<SPIE: Debug> From<SPIE> for Error<SPIE> {
    fn from(e: SPIE) -> Self {
        Error::SpiError(e)
    }
}

/// The driver
pub struct NRF24L01<CE: OutputPin, CSN: OutputPin, SPI: SpiTransfer<u8>, D: DelayUs<u16>> {
    ce: CE,
    csn: CSN,
    spi: SPI,
    delay: D,
    base_config: u8,
}

pub mod setup {
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
}

impl<CE: OutputPin, CSN: OutputPin, SPI: SpiTransfer<u8, Error=SPIE>, SPIE: Debug, D: DelayUs<u16>> NRF24L01<CE, CSN, SPI, D> {
    // Private methods and functions

    fn send_command(&mut self, data_out: &[u8], data_in: &mut [u8]) -> Result<(), Error<SPIE>> {
        data_in.copy_from_slice(data_out);
        self.csn.set_low();
        self.delay.delay_us(2);
        self.spi.transfer(data_in)?;
        self.delay.delay_us(50);
        self.csn.set_high();
        self.delay.delay_us(50);
/*
        let mut stdout = hio::hstdout().unwrap();
        for b in data_out {
            write!(stdout, "{:02X} ", b);
        }
        write!(stdout, ">>");
        for b in data_in {
            write!(stdout, " {:02X}", b);
        }
        writeln!(stdout, "");
*/
        Ok(())
    }

    fn write_register(&mut self, register: Register, byte: u8) -> Result<(), Error<SPIE>> {
        // For single byte registers only
        let mut response_buffer = [0u8; 2];
        self.send_command(&[W_REGISTER | register, byte], &mut response_buffer)
    }

    fn read_register(&mut self, register: Register) -> Result<(u8, u8), Error<SPIE>> {
        // For single byte registers only.
        // Return (STATUS, register)
        let mut response_buffer = [0u8; 2];
        self.send_command(
            &[R_REGISTER | register, 0],
            &mut response_buffer,
        )?;
        Ok((response_buffer[0], response_buffer[1]))
    }

    fn setup_rf(&mut self, rate: DataRate, level: PALevel) -> Result<(), Error<SPIE>> {
        let rate_bits: u8 = match rate {
            DataRate::R250Kbps => 0b0010_0000,
            DataRate::R1Mbps => 0,
            DataRate::R2Mbps => 0b0000_1000,
        };
        let level_bits: u8 = match level {
            PALevel::Min => 0,
            PALevel::Low => 0b0000_0010,
            PALevel::High => 0b0000_0100,
            PALevel::Max => 0b0000_0110,
        };
        self.write_register(RF_SETUP, rate_bits | level_bits)
    }

    fn set_channel(&mut self, channel: u8) -> Result<(), Error<SPIE>> {
        if channel < 126 {
            self.write_register(RF_CH, channel)
        } else {
            self.write_register(RF_CH, 125)
        }
    }

    fn set_full_address(&mut self, pipe: Register, address: &[u8]) -> Result<(), Error<SPIE>> {
        let mut response_buffer = [0u8; 6];
        let mut command = [W_REGISTER | pipe, 0, 0, 0, 0, 0];
        let len = 1 + address.len();
        command[1..len].copy_from_slice(&address);
        self.send_command(&command[0..len], &mut response_buffer[0..len])
    }

    fn set_auto_ack(&mut self, pipes_auto_ack: [bool; 6]) -> Result<(), Error<SPIE>> {
        let mut register = 0;
        for (i, auto_ack) in pipes_auto_ack.iter().enumerate() {
            if *auto_ack {
                register |= 1 << i;
            }
        }
        // auto acknowlegement
        self.write_register(EN_AA, register)
    }

    fn configure_receiver(&mut self, config: &RXConfig) -> Result<u8, Error<SPIE>> {
        // set data rate
        // set PA level
        self.setup_rf(config.data_rate, config.pa_level)?;
        // set channel
        self.set_channel(config.channel)?;
        // set address width
        let aw = config.address_width;
        assert!(aw >= 3 && aw <= 5);
        self.write_register(SETUP_AW, aw as u8 - 2)?;
        // set Pipe 0 address
        self.set_full_address(RX_ADDR_P0, &config.pipe0_address[0..aw])?;
        let mut enabled = 1u8;
        // Pipe 1
        if let Some(address) = config.pipe1_address {
            self.set_full_address(RX_ADDR_P1, &address[0..aw])?;
            enabled |= 0b0000_0010
        };
        // Pipe 2
        if let Some(lsb) = config.pipe2_addr_lsb {
            self.write_register(RX_ADDR_P2, lsb)?;
            enabled |= 0b0000_0100
        };
        // Pipe 3
        if let Some(lsb) = config.pipe3_addr_lsb {
            self.write_register(RX_ADDR_P3, lsb)?;
            enabled |= 0b0000_1000
        }
        // Pipe 4
        if let Some(lsb) = config.pipe4_addr_lsb {
            self.write_register(RX_ADDR_P4, lsb)?;
            enabled |= 0b0001_0000
        };
        // Pipe 5
        if let Some(lsb) = config.pipe5_addr_lsb {
            self.write_register(RX_ADDR_P5, lsb)?;
            enabled |= 0b0010_0000
        };
        // Configure dynamic payload lengths
        let mut feature = 0;
        if config.pipes_static_packet_len.iter().any(|pipe_packet_len| pipe_packet_len.is_some()) {
            feature |= FEATURE_EN_DPL;
        }
        if config.pipes_auto_ack.iter().any(|pipe_auto_ack| *pipe_auto_ack) {
            feature |= FEATURE_EN_ACK_PAY;
        }
        self.write_register(FEATURE, feature)?;
        // Configure static payload lengths
        let mut dynpd = 0;
        for (i, c) in config.pipes_static_packet_len.iter().enumerate() {
            match *c {
                Some(len) => {
                    assert!(len < (1 << 6));
                    self.write_register(RX_PW[i], len)?;
                }
                None => {
                    // Enable dynamic payload length
                    dynpd |= 1 << i;
                }
            }
        }
        self.write_register(DYNPD, dynpd)?;
        // Configure Auto-ack
        self.set_auto_ack(config.pipes_auto_ack)?;
        // Enable configured pipes
        self.write_register(EN_RXADDR, enabled)?;
        // base config is 2 bytes for CRC and RX mode on
        // only reflect RX_DR on the IRQ pin
        let mut base_config = 0b0111_0000 | CONFIG_PRIM_RX;
        config.crc_mode.map(|crc_mode| {
            base_config |= crc_mode.config_mask();
        });

        Ok(base_config)
    }

    fn configure_transmitter(&mut self, config: &TXConfig) -> Result<u8, Error<SPIE>> {
        // set data rate
        // set PA level
        self.setup_rf(config.data_rate, config.pa_level)?;
        // set channel
        self.set_channel(config.channel)?;
        // set address width
        let aw = config.address_width;
        assert!(aw >= 3 && aw <= 5);
        self.write_register(SETUP_AW, aw as u8 - 2)?;
        // set destination and Pipe 0 address
        self.set_full_address(RX_ADDR_P0, &config.pipe0_address[0..aw])?;
        self.set_full_address(TX_ADDR, &config.pipe0_address[0..aw])?;
        // disable other pipes
        self.write_register(EN_RXADDR, 1u8)?;
        // retransmission settings
        let retry_bits: u8 = config.max_retries.min(15);
        let retry_delay_bits: u8 = config.retry_delay.min(0xF);
        self.write_register(
            SETUP_RETR,
            (retry_delay_bits << 4) | retry_bits,
        )?;
        // base config is 2 bytes for CRC and TX mode on
        // only reflect TX_DS and MAX_RT on the IRQ pin
        let mut base_config = 0b0111_0000;
        config.crc_mode.map(|crc_mode| {
            base_config |= crc_mode.config_mask();
        });

        Ok(base_config)
    }

    // Public API

    /// Construct a new driver instance.
    ///
    /// # Errors
    ///
    /// System IO errors
    ///
    pub fn new(mut ce: CE, mut csn: CSN, spi: SPI, delay: D) -> Result<Self, Error<SPIE>> {
        ce.set_low();
        csn.set_high();

        Ok(NRF24L01 {
            ce, csn, spi, delay,
            base_config: 0b0000_0000,
        })
    }

    /// Configure the device as Primary Receiver (PRX) or Primary Transmitter (PTX),
    /// set all its properties for proper operation and power it up.
    ///
    /// The device remain in standby until `self.listen()` (RX mode)
    /// or `self.send()` (TX mode) is called.
    ///
    /// All commands work when the device is in standby (recommended) as well as
    /// active state.
    pub fn configure(&mut self, mode: &OperatingMode) -> Result<(), Error<SPIE>> {
        self.ce.set_low();

        // Mode specific configuration
        match *mode {
            OperatingMode::RX(ref config) => self.configure_receiver(config),
            OperatingMode::TX(ref config) => self.configure_transmitter(config),
        }.and_then(|base_config| {
            // Reset status
            self.write_register(STATUS, STATUS_RX_DR | STATUS_TX_DS | STATUS_MAX_RT);
            // Go!
            self.base_config = base_config;
            self.power_up()?;
            self.delay.delay_us(150);
            Ok(())
        })
    }

    pub fn is_receiver(&self) -> bool {
        self.base_config & 1u8 == 1u8
    }

    /// Power down the device.
    ///
    /// The power consumption is minimum in this mode, and the device ceases all operation.
    /// It only accepts configuration commands.
    pub fn power_down(&mut self) -> Result<(), Error<SPIE>> {
        self.ce.set_low();
        self.base_config &= !CONFIG_PWR_UP;
        let base_config = self.base_config;
        self.write_register(CONFIG, base_config)
    }

    /// Power the device up for full operation.
    pub fn power_up(&mut self) -> Result<(), Error<SPIE>> {
        self.base_config |= CONFIG_PWR_UP;
        let base_config = self.base_config;
        self.write_register(CONFIG, base_config)
    }

    /// Put the device in standby (RX Mode)
    ///
    /// Only used in RX mode to suspend active listening.
    /// In TX mode, standby is the default state when not sending data.
    pub fn standby(&mut self) -> Result<(), Error<SPIE>> {
        self.ce.set_low(); // always returnss without error.
        Ok(())
    }

    /// (RX mode only) Wake up and activate receiver.
    ///
    /// In RX mode, call this function after a `.configure(...)`, `.standby()` or `power_up()` to
    /// accept incoming packets.
    pub fn listen(&mut self) -> Result<(), Error<SPIE>> {
        if self.is_receiver() {
            self.ce.set_high();
        }
        Ok(())
    }

    pub fn is_connected(&mut self) -> Result<bool, Error<SPIE>> {
        self.read_register(SETUP_AW)
            .map(|(_, setup_aw)| setup_aw >= 1 && setup_aw <= 3)
    }

    /// Is there any incoming data to read? Return the pipe number.
    ///
    /// Works in RX mode.
    pub fn data_available(&mut self) -> Result<Option<u8>, Error<SPIE>> {
        self.read_register(FIFO_STATUS).map(
            |(status, fifo_status)| {
                // let mut stdout = hio::hstdout().unwrap();
                // writeln!(stdout, "status={:02X}\tfifo_status={:02X}", status, fifo_status);
                if fifo_status & FIFO_STATUS_RX_EMPTY == 0 {
                    let rx_p_no = status & STATUS_RX_P_NO;
                    Some(rx_p_no >> 1)
                } else {
                    None
                }
            },
        )
    }

    /// Read data from the receiver queue, one packet at a time.
    ///
    /// The `process_packet` callback is fired for each packet, and is
    /// passed a slice into the packet data as argument.
    ///
    /// ``read_all`` returns the number of messages read.
    ///
    /// **Note**: this function puts the device in standby mode during
    /// the processing of the queue and restores operations when it returns *successfully*.
    /// So the `process_packet` callback should better return quickly.
    pub fn read_all<F>(&mut self, mut process_packet: F) -> Result<u8, Error<SPIE>>
    where
        F: FnMut(&[u8]) -> (),
    {
        // communication buffers
        let mut pl_wd: [u8; 2] = [0, 0]; // for packet width
        let mut receive_buffer = [0u8; 33]; // for packet
        let out_buffer = [R_RX_PAYLOAD; 33]; // for command
        // message counter
        let mut count = 0u8;
        // save CE state
        let ce_state = self.ce.is_high();
        // Standby
        self.ce.set_low();
        // process queue
        while let Some(pipe_no) = self.data_available()? {
            let mut stdout = hio::hstdout().unwrap();
            writeln!(stdout, "Read on pipe {}", pipe_no);

            self.send_command(&[R_RX_PL_WID, 0xff], &mut pl_wd)?;
            let width = pl_wd[1] as usize;
            // if width < 1 {
            //     break;
            // }
            let ubound = (width + 1).min(33);
            self.send_command(
                &out_buffer[..ubound],
                &mut receive_buffer[..ubound],
            )?;
            process_packet(&receive_buffer[1..ubound]);
            count += 1;
        }
        // Clear interrupt
        self.write_register(STATUS, 0b0100_0000)?;
        // Restore previous CE state
        if ce_state {
            self.ce.set_high();
        }
        Ok(count)
    }

    /// Queue (FIFO) data to be sent, one packet at a time.
    ///
    /// In TX mode, `pipe_num` is ignored. In RX mode, this function queues an ACK payload
    /// for the next message to arrive on `pipe_num`. if `pipe_num` is bigger than 5,
    /// it is capped to 5.
    ///
    /// The maximum size for a packet is 32 bytes.
    ///
    /// You can store a maximun of 3 payloads in the FIFO queue.
    ///
    /// **Note** : The payloads remain in the queue until the end of an _Enhanced Shockburst_ ™
    /// transaction:
    ///
    /// * In TX mode, a payload is removed from the send queue if and only if
    /// it has been successfully sent, that is, an ACK (with or without payload) has
    /// been received for it.
    ///
    /// * In RX mode, an ACK payload is removed from the queue if and only if
    /// it has been sent AND the pipe receives a new message, different from
    /// the one the ACK payload responded to. This is because the receiver has
    /// no mean to know whether the transmitter has received the ACK until
    /// it receives a new, different message from the same transmitter.
    /// So, it keeps the ACK payload under hand in case the transmitter resends the same
    /// packet over again.
    pub fn push(&mut self, pipe_num: u8, data: &[u8]) -> Result<(), Error<SPIE>> {
        let (status, fifo_status) = self.read_register(FIFO_STATUS)?;
        if (status & 1 != 0) || (fifo_status & 0b0010_0000 != 0) {
            // TX_FIFO is full
            // TODO
            panic!("Sending queue is full!")
        } else {
            let command = if self.is_receiver() {
                let actual_pipe_num: u8 = if pipe_num < 6 { pipe_num } else { 5 };
                W_ACK_PAYLOAD | actual_pipe_num
            } else {
                W_TX_PAYLOAD
            };
            if data.len() > 32 {
                // TODO
                panic!("Packet too big!")
            } else {
                let mut out_buffer = [command; 33];
                let ubound = data.len() + 1;
                out_buffer[1..ubound].copy_from_slice(data);
                let mut in_buffer = [0u8; 33];
                self.send_command(&out_buffer[..ubound], &mut in_buffer[..ubound])
            }
        }
    }

    /// [TX mode only] Send all packets in the TX FIFO queue.
    ///
    /// The call blocks until all packets are sent or the device reaches
    /// the `max_retries` number of retries after failure.
    /// Return the number of retries in case of success.
    ///
    /// The payloads that failed to be sent remain in the TX queue.
    /// You can call `.send()` again to relaunch a send/retry cycle or
    /// call `.flush_output` to clear the queue.
    pub fn send(&mut self) -> Result<(), Error<SPIE>> {
        // let mut stdout = hio::hstdout().unwrap();
        // let r = self.read_register(CONFIG)?;
        // writeln!(stdout, "config={:02X}", r.1);

        let mut status = self.read_register(FIFO_STATUS)?;
        while status.1 & FIFO_STATUS_TX_EMPTY == 0 {
            // writeln!(stdout, "status={:02X}\tfifo_status={:02X}", status.0, status.1);
            self.ce.set_high();
            self.delay.delay_us(10);
            self.ce.set_low();
            // clear TX_DS and MAX_RT
            self.write_register(STATUS, STATUS_TX_DS | STATUS_MAX_RT)?;
            self.delay.delay_us(10);
            status = self.read_register(FIFO_STATUS)?;
        }
        self.ce.set_low();
        Ok(())
    }

    /// Clear input queue.
    ///
    /// In RX mode, use only when device is in standby.
    pub fn flush_input(&mut self) -> Result<(), Error<SPIE>> {
        let mut buffer = [0u8];
        self.send_command(&[FLUSH_RX], &mut buffer)?;
        Ok(())
    }

    /// Clear output queue.
    ///
    /// In RX mode, use only when device is in standby.
    pub fn flush_output(&mut self) -> Result<(), Error<SPIE>> {
        let mut buffer = [0u8];
        self.send_command(&[FLUSH_TX], &mut buffer)?;
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rx_defaults() {
        let rx_conf = RXConfig::default();
        assert_eq!(rx_conf.data_rate, DataRate::R1Mbps);
        assert_eq!(rx_conf.channel, 0);
        assert_eq!(rx_conf.pa_level, PALevel::Min);
        assert_eq!(rx_conf.pipe0_address, [0u8; 5]);
        assert_eq!(rx_conf.pipe1_address, None);
    }

    #[test]
    fn rx_partial_defaults() {
        let mut rx_conf = RXConfig {
            channel: 108,
            data_rate: DataRate::R250Kbps,
            pa_level: PALevel::Low,
            pipe0_address: *b"rxadd",
            ..Default::default()
        };
        rx_conf.pipe0_address.reverse();
        assert_eq!(rx_conf.channel, 108);
        assert_eq!(rx_conf.pipe1_address, None);
    }

    #[test]
    fn tx_defaults() {
        let tx_conf = TXConfig::default();
        assert_eq!(tx_conf.data_rate, DataRate::R1Mbps);
        assert_eq!(tx_conf.channel, 0);
        assert_eq!(tx_conf.pa_level, PALevel::Min);
        assert_eq!(tx_conf.max_retries, 0);
        assert_eq!(tx_conf.retry_delay, 0);
        assert_eq!(tx_conf.pipe0_address, [0u8; 5]);
    }
}
