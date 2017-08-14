//! A pure Rust driver for NRF24L01 transceivers
//!
//! The aim of this driver is to provide a rustic, easy to use, no non-sense
//! API to drive an NRF24L01 transceiver.This is not a port from a C or C++ library.
//! It has been written from scratch based on the
//! [specs](https://duckduckgo.com/l/?kh=-1&uddg=https%3A%2F%2Fwww.sparkfun.com%2Fdatasheets%2FComponents%2FSMD%2FnRF24L01Pluss_Preliminary_Product_Specification_v1_0.pdf).
//!
//! For the moment, the driver only offer an API for the most reliable communication
//! scheme offered by NRF24L01 chips, that is _Enhanced Shockburst™_, with
//! automatic (hardware) packet acknowlegement with optional payload, dynamic payload length and
//! long CRC (2 bytes).


extern crate spidev;
extern crate sysfs_gpio;

use std::io;


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


/// Supported power amplifier levels.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum PALevel {
    /// -18 dBm, 7.0 mA DC current consumption, few meters range.
    Min,
    /// -12 dBM, 7.5 mA DC current consumption.
    Low,
    /// -6 dBm, 9.0 mA DC current consumption.
    High,
    /// 0 dBm, 11.3 mA DC current consumption, up to 100 meters.
    Max,
}

impl Default for PALevel {
    fn default() -> PALevel {
        PALevel::Min
    }
}

/// Receiver mode configuration
#[derive(Debug)]
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
    /// Pipe 0 address
    ///
    /// This is the receiving base address.
    ///
    /// Typically, this is the only address you need to set, unless you
    /// need a multiceiver configuration. In that case, you can enable up to
    /// five additional receiving pipes.
    ///
    /// The address is in big endian order: the first byte is the least significant one.
    ///
    /// You must provide an valid address for Pipe 0.
    ///
    /// All pipes 2-5 share the 4 most significant bytes with the pipe 1 address, so
    /// you only need to provide the least significant byte to enable one of those pipes or
    /// set it to None to disable it (default).
    pub pipe0_address: [u8; 5],
    /// Pipe 1 address, defaults to None (disabled)
    pub pipe1_address: Option<[u8; 5]>,
    /// Pipe 2 LSB, defaults to None (disabled)
    pub pipe2_addr_lsb: Option<u8>,
    /// Pipe 3 LSB, defaults to None (disabled)
    pub pipe3_addr_lsb: Option<u8>,
    /// Pipe 4 LSB, defaults to None (disabled)
    pub pipe4_addr_lsb: Option<u8>,
    /// Pipe 5 LSB, defaults to None (disabled)
    pub pipe5_addr_lsb: Option<u8>,
}


/// Transmitter mode configuration
#[derive(Debug)]
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
    /// 0 <= `retry_delay` <= 15. Default is 0, recommended is > 1. Any value above 15 is capped to 15.
    pub retry_delay: u8, // [0, 15]
    /// Destination address, should match an address on the receiver end.
    ///
    /// This is also the address on which ACK packets are received.
    /// The address is in big endian order: the first byte is the least significant one.
    pub pipe0_address: [u8; 5],
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
// Nop, maybe used to just read the status register
const NOP: Command = 0b1111_1111;
// Read input FIFO
const R_RX_PAYLOAD: Command = 0b0110_0001;
// Read the size of the packet on top of input FIFO
const R_RX_PL_WID: Command =  0b0110_0000;

type Register = u8;

// Base config, p 54
const CONFIG: Register = 0;
// Enable auto acknowlegment, p54
const EN_AA: Register = 0x01;
// Enabled RX addresses, p 54
const EN_RXADDR: Register = 0x02;
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
// Destination address, p 57
const TX_ADDR: Register = 0x10;
// FIFO status (RX & TX), p 58
const FIFO_STATUS: Register = 0x17;
// Enable dynamic payload length (requires EN_DPL and ENAA_PX), p 59
const DYNPD: Register = 0x1C;
//  Feature register (content EN_DPL, EN_ACK_PAY...), p 59
const FEATURE: Register = 0x1D;



/// The driver
pub struct NRF24L01 {
    ce: sysfs_gpio::Pin,
    spi: spidev::Spidev,
    base_config: u8
}

impl NRF24L01 {

    // Private methods and functions

    fn send_command(&self, data_out: &[u8], data_in: &mut [u8]) -> io::Result<()> {
        let mut transfer = spidev::SpidevTransfer::read_write(data_out, data_in);
        self.spi.transfer(&mut transfer)
    }

    fn write_register(&self, register: Register, byte: u8) -> io::Result<()> {
        // For single byte registers only
        let mut response_buffer = [0u8; 2];
        self.send_command(&[W_REGISTER | register, byte], &mut response_buffer)
    }

    fn setup_rf(&self, rate: DataRate, level: PALevel) -> io::Result<()> {
        let rate_bits: u8 = match rate {
            DataRate::R250Kbps => 0b00100000,
            DataRate::R1Mbps => 0,
            DataRate::R2Mbps => 0b000001000,
        };
        let level_bits: u8 = match level {
            PALevel::Min => 0,
            PALevel::Low => 0b00000010,
            PALevel::High => 0b00000100,
            PALevel::Max => 0b00000110,
        };
        self.write_register(RF_SETUP, rate_bits | level_bits)
    }

    fn set_channel(&self, channel: u8) -> io::Result<()> {
        if channel < 126 {
            self.write_register(RF_CH, channel)
        } else {
            self.write_register(RF_CH, 125)
        }
    }

    fn set_full_address(&self, pipe: Register, address: [u8;5])  -> io::Result<()> {
        let mut response_buffer = [0u8;6];
        let mut command = [W_REGISTER | pipe, 0, 0, 0, 0, 0];
        command[1..].copy_from_slice(&address);
        self.send_command(&command, &mut response_buffer)
    }

    fn configure_receiver(&self, config: &RXConfig) -> io::Result<u8> {
        // set data rate
        // set PA level
        self.setup_rf(config.data_rate, config.pa_level)?;
        // set channel
        self.set_channel(config.channel)?;
        // set Pipe 0 address
        self.set_full_address(RX_ADDR_P0, config.pipe0_address)?;
        let mut enabled = 1u8;
        // Pipe 1
        if let Some(address) = config.pipe1_address {
            self.set_full_address(RX_ADDR_P1, address)?;
            enabled |= 0b00000010
        };
        // Pipe 2
        if let Some(lsb) = config.pipe2_addr_lsb {
            self.write_register(RX_ADDR_P2, lsb)?;
            enabled |= 0b00000100
        };
        // Pipe 3
        if let Some(lsb) = config.pipe3_addr_lsb {
            self.write_register(RX_ADDR_P3, lsb)?;
            enabled |= 0b00001000
        }
        // Pipe 4
        if let Some(lsb) = config.pipe4_addr_lsb {
            self.write_register(RX_ADDR_P4, lsb)?;
            enabled |= 0b00010000
        };
        // Pipe 5
        if let Some(lsb) = config.pipe5_addr_lsb {
            self.write_register(RX_ADDR_P5, lsb)?;
            enabled |= 0b00100000
        };
        // Enable configured pipes
        self.write_register(EN_RXADDR, enabled)?;
        // base config is 2 bytes for CRC and RX mode on.
        Ok(0b00001101)
    }

    fn configure_transmitter(&self, config: &TXConfig) -> io::Result<u8> {
        // set data rate
        // set PA level
        self.setup_rf(config.data_rate, config.pa_level)?;
        // set channel
        self.set_channel(config.channel)?;
        // set destination and Pipe 0 address
        self.set_full_address(RX_ADDR_P0, config.pipe0_address)?;
        self.set_full_address(TX_ADDR, config.pipe0_address)?;
        // disable other pipes
        self.write_register(EN_RXADDR, 1u8)?;
        // retransmission settings
        let retry_bits: u8 = if config.max_retries < 16 { config.max_retries } else { 15 };
        let retry_delay_bits: u8 = if config.retry_delay < 16 { config.retry_delay << 4 } else { 0xF0 };
        self.write_register(SETUP_RETR, retry_delay_bits | retry_bits)?;
        // base config is 2 bytes for CRC and TX mode on.
        Ok(0b00001100)
    }

    // Public API

    /// Construct a new driver instance.
    ///
    /// * `ce_pin`: the GPIO number (Linux SysFS) connected to the CE pin of the transceiver
    /// * `spi_device`: the SPI device number (or channel) the transceiver is connected to.
    ///
    /// We use the spidev linux kernel driver.
    ///
    /// # Errors
    ///
    /// System IO errors
    ///
    pub fn new(ce_pin: u64, spi_device: u8) -> io::Result<NRF24L01> {
        let mut spi = try!(spidev::Spidev::open(format!("/dev/spidev0.{}", spi_device)));
        let options = spidev::SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(10_000_000)
            .mode(spidev::SPI_MODE_0)
            .build();
        spi.configure(&options)?;
        let ce = sysfs_gpio::Pin::new(ce_pin);
        ce.export().map_err(|_| {
            io::Error::new(io::ErrorKind::PermissionDenied, "Unable to export CE")
        })?;
        ce.set_direction(sysfs_gpio::Direction::Low).map_err(|_| {
            io::Error::new(io::ErrorKind::PermissionDenied, "Unable to set CE")
        })?;
        Ok(NRF24L01 { ce, spi, base_config: 0b00001101 })
    }

    /// Configure the device as Primary Receiver (PRX) or Primary Transmitter (PTX),
    /// set all its properties for proper operations and power it up.
    ///
    /// The device remain in standby until `self.listen()` (RX mode) or `self.send()` (TX mode) is called.
    pub fn configure(&mut self, mode: &OperatingMode) -> io::Result<()> {
        self.ce.set_value(0).unwrap();
        // auto acknowlegement
        self.write_register(EN_AA, 0b00111111)?;
        // dynamic payload and payload with ACK
        self.write_register(DYNPD, 0b00111111)?;
        self.write_register(FEATURE, 0b00000110)?;

        // Mode specific configuration
        let config_result = match mode {
            &OperatingMode::RX(ref config) => self.configure_receiver(config),
            &OperatingMode::TX(ref config) => self.configure_transmitter(config),
        };
        if let Ok(base_config) = config_result {
            // Go!
            self.base_config = base_config;
            self.power_up()
        } else {
            // return error
            config_result.map(|_| ())
        }
    }

    pub fn is_receiver(&self) -> bool {
        self.base_config & 1u8 == 1u8
    }

    /// Power down the device.
    ///
    /// The power consumption is minimum in this mode, and the device ceases all operation.
    /// It only accepts configuration commands.
    pub fn power_down(&self) -> io::Result<()> {
        self.ce.set_value(0).unwrap();
        self.write_register(CONFIG, self.base_config)
    }

    /// Power the device up for full operation.
    pub fn power_up(&self) -> io::Result<()> {
        self.write_register(CONFIG, self.base_config | 0b00000010)
    }

    /// Put the device in standby (RX Mode)
    ///
    /// Only used in RX mode to suspend active listening.
    /// In TX mode, standby is the default state when not sending data.
    pub fn standby(&self) -> io::Result<()> {
        self.ce.set_value(0).unwrap(); // always returnss without error.
        Ok(())
    }

    /// (RX mode only) Wake up and activate receiver.
    ///
    /// In RX mode, call this function after a `.configure(...)`, `.standby()` or `power_up()` to
    /// accept incoming packets.
    pub fn listen(&self) -> io::Result<()> {
        if self.is_receiver() {
            self.ce.set_value(1).unwrap()
        }
        Ok(())
    }


    /// Is there any incoming data to read?
    ///
    /// Works in both RX and TX modes. In TX mode, this function returns true if
    /// a ACK payload has been received.
    pub fn data_available(&self) -> io::Result<bool> {
        // TODO: should we return the number of the pipe that received the last packet?
        let mut registers = [0, 0]; // STATUS, FIFO_STATUS
        self.send_command(&[R_REGISTER | FIFO_STATUS, 0], &mut registers)?;
        Ok((registers[0] & 0b01000000 != 0) || (registers[1] & 1u8 == 0))
    }

    /// Read incoming data, one packet at a time.
    ///
    /// Return the packet length if any.
    /// Check `self.data_available()` for additional data to read.
    pub fn read(&self, buffer: &mut [u8;32]) -> io::Result<u8> {
        let mut pl_wd = [0u8];
        self.send_command(&[R_RX_PL_WID], &mut pl_wd)?;
        let width = pl_wd[0];
        if width != 0 {
            let mut receive_buffer = [0u8; 33];
            self.send_command(&[R_RX_PAYLOAD; 33], &mut receive_buffer)?;
            // Clear interrupt
            self.write_register(STATUS, 0b01000000)?;
            buffer.copy_from_slice(&receive_buffer[1..]);
            Ok(width)
        } else {
            Ok(0)
        }
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
