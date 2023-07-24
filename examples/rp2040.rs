#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use panic_probe as _;
use defmt_rtt as _;

use core::cell::RefCell;

use nrf24l01::{TXConfig, PALevel, NRF24L01, OperatingMode};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_sync::blocking_mutex::{Mutex, raw::NoopRawMutex};
use embassy_executor::Spawner;
use embassy_rp::{
    spi::Spi,
    gpio::{Level, Output, Pin}, spi::Blocking
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
    // SPI
    let mut spi_config = embassy_rp::spi::Config::default();
    spi_config.frequency = 10_000_000; // TODO: is this right?
    spi_config.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition; // TODO: is this right?
    spi_config.polarity = embassy_rp::spi::Polarity::IdleHigh; // TODO: is this right?
    let clk = peripherals.PIN_10;
    let mosi = peripherals.PIN_11;
    let miso =  peripherals.PIN_12;
    let cs = peripherals.PIN_9;
    let cs_output = Output::new(cs, Level::High);
    let spi: Spi<'_, _, Blocking> = Spi::new_blocking(peripherals.SPI1, clk, mosi, miso, spi_config.clone());
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));
    let spi_device = SpiDeviceWithConfig::new(&spi_bus, cs_output, spi_config);
    // NRF24L01
    let nrf24l01_config = TXConfig {
        channel: 108,
        pa_level: PALevel::Low,
        pipe0_address: *b"abcde",
        max_retries: 3,
        retry_delay: 2,
        ..Default::default()
    };
    let mut device = NRF24L01::new(spi_device, cs_output).unwrap(); // TODO: how to share cs_output with SpiDeviceWithConfig because it gets moved?
    let message = b"sendtest";
    device.configure(&OperatingMode::TX(nrf24l01_config)).unwrap();
    device.flush_output().unwrap();
    loop {
        blink(&mut led_output).await;
        // spi
        device.push(0, message).unwrap();
        match device.send() {
            Ok(retries) => defmt::info!("Message sent, {} retries needed", retries),
            Err(_err) => {
                defmt::error!("Destination unreachable"); // TODO: print/format err somehow?
                device.flush_output().unwrap()
            }
        };
        Timer::after(Duration::from_millis(5000)).await;
    }
}