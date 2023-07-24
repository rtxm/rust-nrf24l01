#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use nrf24l01::{TXConfig, PALevel, NRF24L01, OperatingMode, RXConfig};
use panic_probe as _;
use defmt_rtt as _;

use core::cell::RefCell;

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_sync::blocking_mutex::{Mutex, raw::NoopRawMutex};
use embassy_executor::Spawner;
use embassy_rp::{
    spi::{Spi, Blocking},
    gpio::{Level, Output, Pin},
};
use embassy_time::{Duration, Timer};

async fn blink<T>(led: &mut Output<'_, T>) where T: Pin {
    led.set_high();
    Timer::after(Duration::from_millis(100)).await;
    led.set_low();
    Timer::after(Duration::from_millis(100)).await;
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // peripherals
    let peripherals_config = embassy_rp::config::Config::default();
    let peripherals = embassy_rp::init(peripherals_config);
    // LED
    let mut led_output = Output::new(peripherals.PIN_25, Level::Low);
    // SPI0
    let mut spi0_config = embassy_rp::spi::Config::default();
    spi0_config.frequency = 10_000_000; // TODO: is this right?
    spi0_config.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition; // TODO: is this right?
    spi0_config.polarity = embassy_rp::spi::Polarity::IdleHigh; // TODO: is this right?
    let spi0_clk = peripherals.PIN_18;
    let spi0_mosi = peripherals.PIN_19;
    let spi0_miso =  peripherals.PIN_20;
    let spi0_cs = peripherals.PIN_21;
    let spi0_cs_output = Output::new(spi0_cs, Level::High);
    let spi0: Spi<'_, _, Blocking> = Spi::new_blocking(peripherals.SPI0, spi0_clk, spi0_mosi, spi0_miso, spi0_config.clone());
    let spi0_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi0));
    let spi0_device = SpiDeviceWithConfig::new(&spi0_bus, spi0_cs_output, spi0_config);
    // SPI1
    let mut spi1_config = embassy_rp::spi::Config::default();
    spi1_config.frequency = 10_000_000; // TODO: is this right?
    spi1_config.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition; // TODO: is this right?
    spi1_config.polarity = embassy_rp::spi::Polarity::IdleHigh; // TODO: is this right?
    let sp1_clk = peripherals.PIN_10;
    let sp1_mosi = peripherals.PIN_11;
    let sp1_miso =  peripherals.PIN_12;
    let sp1_cs = peripherals.PIN_13;
    let sp1_cs_output = Output::new(sp1_cs, Level::High);
    let spi1: Spi<'_, _, Blocking> = Spi::new_blocking(peripherals.SPI1, sp1_clk, sp1_mosi, sp1_miso, spi1_config.clone());
    let spi1_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi1));
    let spi1_device = SpiDeviceWithConfig::new(&spi1_bus, sp1_cs_output, spi1_config);
    // NRF24L01P transmitter
    let tx_ce = peripherals.PIN_0;
    let tx_ce_output = Output::new(tx_ce, Level::Low);
    let tx_config = TXConfig {
        channel: 108,
        pa_level: PALevel::Low,
        pipe0_address: *b"abcde",
        max_retries: 3,
        retry_delay: 2,
        ..Default::default()
    };
    let mut nrf24l01_tx_device = NRF24L01::new(spi0_device, tx_ce_output).unwrap();
    let message = b"sendtest";
    nrf24l01_tx_device.configure(&OperatingMode::TX(tx_config)).unwrap();
    nrf24l01_tx_device.flush_output().unwrap();
    // NRF24L01P receiver
    let rx_ce = peripherals.PIN_22;
    let rx_ce_output = Output::new(rx_ce, Level::Low);
    let rx_config = RXConfig {
        channel: 108,
        pa_level: PALevel::Low,
        pipe0_address: *b"abcde",
        ..Default::default()
    };
    let mut nrf24l01_rx_device = NRF24L01::new(spi1_device, rx_ce_output).unwrap();
    nrf24l01_rx_device.configure(&OperatingMode::RX(rx_config)).unwrap();
    nrf24l01_rx_device.listen().unwrap();
    loop {
        // blink
        blink(&mut led_output).await;
        // transmit
        nrf24l01_tx_device.push(0, message).unwrap();
        match nrf24l01_tx_device.send() {
            Ok(retries) => defmt::info!("Message sent, {} retries needed", retries),
            Err(_err) => {
                defmt::error!("Destination unreachable"); // TODO: fmt error?
                nrf24l01_tx_device.flush_output().unwrap()
            }
        };
        // receive
        if nrf24l01_rx_device.data_available().unwrap() {
            nrf24l01_rx_device
                .read_all(|packet| {
                    defmt::info!("Received {:?} bytes", packet.len());
                    defmt::info!("Payload {:?}", packet);
                })
                .unwrap();
        }
        // sleep
        Timer::after(Duration::from_millis(1000)).await;
    }
}
