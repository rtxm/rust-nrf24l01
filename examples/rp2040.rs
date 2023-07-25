//! This example shows how to send messages between the two cores in the RP2040 chip.
//!
//! The LED on the RP Pico W board is connected differently. See wifi_blinky.rs.

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::cell::RefCell;

use defmt::*;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use embassy_executor::Executor;
use embassy_rp::gpio::Level;
use embassy_rp::gpio::Output;
use embassy_rp::peripherals::*;
use embassy_rp::spi::Blocking;
use embassy_rp::spi::Spi;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Duration, Timer};
use nrf24l01::DataRate;
use nrf24l01::NRF24L01;
use nrf24l01::OperatingMode;
use nrf24l01::PALevel;
use nrf24l01::RXConfig;
use nrf24l01::TXConfig;
use static_cell::StaticCell;
use panic_probe as _;
use defmt_serial as _;
use embassy_rp::uart;
use embassy_rp::bind_interrupts;
use embassy_rp::uart::InterruptHandler;
use embassy_rp::peripherals::UART0;
use static_cell::make_static;

static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static SPI0_BUS: StaticCell<Mutex<NoopRawMutex, RefCell<Spi<'_, SPI0, Blocking>>>> = StaticCell::new();
static SPI1_BUS: StaticCell<Mutex<NoopRawMutex, RefCell<Spi<'_, SPI1, Blocking>>>> = StaticCell::new();

bind_interrupts!(struct Irqs {
    UART0_IRQ => InterruptHandler<UART0>;
});

#[cortex_m_rt::entry]
fn main() -> ! {
    // peripherals
    let p = embassy_rp::init(Default::default());
    // UART0
    let uart0_rx = p.PIN_17; // 22, UART0 RX (white)
    let uart0_tx = p.PIN_16; // 21, UART0 TX (blue)
    let uart0_config = uart::Config::default();
    let uart0 = uart::Uart::new(p.UART0, uart0_tx, uart0_rx, Irqs, p.DMA_CH0, p.DMA_CH1, uart0_config);
    // defmt serial
    defmt_serial::defmt_serial(uart0);
    // SPI0
    defmt::info!("setting up SPI0");
    let spi0_clk = p.PIN_18; // 24, SPI0 SCK (yellow)
    let spi0_mosi = p.PIN_19; // 25, SPI0 TX (orange)
    let spi0_miso =  p.PIN_20; // 26, SPI0 RX (green)
    let spi0_cs = p.PIN_21; // 27, SPI0 CSn (purple)
    let tx_ce = p.PIN_22; // 29 (brown)
    let mut spi0_config = embassy_rp::spi::Config::default();
    spi0_config.frequency = 1_000_000; // TODO: is this right?
    spi0_config.phase = embassy_rp::spi::Phase::CaptureOnFirstTransition; // TODO: is this right?
    spi0_config.polarity = embassy_rp::spi::Polarity::IdleLow; // TODO: is this right?
    let spi0_cs_output = Output::new(spi0_cs, Level::Low);
    let spi0 = Spi::new_blocking(p.SPI0, spi0_clk, spi0_mosi, spi0_miso, spi0_config);
    let spi0_bus = Mutex::<NoopRawMutex, _>::new(RefCell::new(spi0));
    let spi0_bus = SPI0_BUS.init(spi0_bus);
    let spi0_device = SpiDevice::new(spi0_bus, spi0_cs_output);
    defmt::info!("set up SPI0");
    // SPI1
    defmt::info!("setting up SPI1");
    let rx_ce = p.PIN_9; // 12 (brown)
    let spi1_clk = p.PIN_10; // 14, SPI1 SCK (yellow)
    let spi1_mosi = p.PIN_11; // 15, SPI1 TX (orange)
    let spi1_miso =  p.PIN_12; // 16, SPI1 RX (green)
    let spi1_cs = p.PIN_13; // 17, SPI1 CSn (purple)
    let mut spi1_config = embassy_rp::spi::Config::default();
    spi1_config.frequency = 1_000_000; // TODO: is this right?
    spi1_config.phase = embassy_rp::spi::Phase::CaptureOnFirstTransition; // TODO: is this right?
    spi1_config.polarity = embassy_rp::spi::Polarity::IdleLow; // TODO: is this right?
    let spi1_cs_output = Output::new(spi1_cs, Level::Low);
    let spi1 = Spi::new_blocking(p.SPI1, spi1_clk, spi1_mosi, spi1_miso, spi1_config);
    let spi1_bus = Mutex::<NoopRawMutex, _>::new(RefCell::new(spi1));
    let spi1_bus = SPI1_BUS.init(spi1_bus);
    let spi1_device = SpiDevice::new(spi1_bus, spi1_cs_output);
    defmt::info!("set up SPI1");
    // NRF24L01P transmitter (SPI0)
    defmt::info!("setting up NRF24L01P transmitter");
    let tx_ce_output = Output::new(tx_ce, Level::Low);
    let nrf24l01_tx_device = NRF24L01::new(spi0_device, tx_ce_output).unwrap();
    defmt::info!("set up NRF24L01P transmitter");
    // NRF24L01P receiver (SPI1)
    defmt::info!("setting up NRF24L01P receiver");
    let rx_ce_output = Output::new(rx_ce, Level::Low);
    let nrf24l01_rx_device = NRF24L01::new(spi1_device, rx_ce_output).unwrap();
    defmt::info!("set up NRF24L01P transmitter");
    // spawn core0
    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        let nrf24l01_tx_device = make_static!(nrf24l01_tx_device);
        let nrf24l01_rx_device = make_static!(nrf24l01_rx_device);
        unwrap!(spawner.spawn(nrf24l01_tx_task(nrf24l01_tx_device)));
        unwrap!(spawner.spawn(nrf24l01_rx_task(nrf24l01_rx_device)));
    });
}

#[embassy_executor::task]
async fn nrf24l01_tx_task(nrf24l01_tx_device: &'static mut NRF24L01<embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice<'_, NoopRawMutex, embassy_rp::spi::Spi<'static, embassy_rp::peripherals::SPI0, embassy_rp::spi::Blocking>, Output<'_, PIN_21>>, Output<'_, PIN_22>>) {
    let tx_config = TXConfig {
        data_rate: DataRate::R1Mbps,
        channel: 108,
        pa_level: PALevel::Low,
        pipe0_address: *b"abcde",
        max_retries: 3,
        retry_delay: 2,
        ..Default::default()
    };
    let message = b"sendtest";
    nrf24l01_tx_device.configure(&OperatingMode::TX(tx_config)).unwrap();
    nrf24l01_tx_device.flush_output().unwrap();
    loop {
        nrf24l01_tx_device.push(0, message).unwrap();
        match nrf24l01_tx_device.send() {
            Ok(retries) => defmt::info!("Message sent, {} retries needed", retries),
            Err(err) => {
                defmt::info!("Destination unreachable: {:?}", err);
                nrf24l01_tx_device.flush_output().unwrap()
            }
        };
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::task]
async fn nrf24l01_rx_task(nrf24l01_rx_device: &'static mut NRF24L01<embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice<'_, NoopRawMutex, embassy_rp::spi::Spi<'static, embassy_rp::peripherals::SPI1, embassy_rp::spi::Blocking>, Output<'_, PIN_13>>, Output<'_, PIN_9>>) {
    let rx_config = RXConfig {
        data_rate: DataRate::R1Mbps,
        channel: 108,
        pa_level: PALevel::Low,
        pipe0_address: *b"abcde",
        ..Default::default()
    };
    nrf24l01_rx_device.configure(&OperatingMode::RX(rx_config)).unwrap();
    nrf24l01_rx_device.listen().unwrap();
    loop {
        Timer::after(Duration::from_secs(1)).await;
        if nrf24l01_rx_device.data_available().unwrap() {
            nrf24l01_rx_device
                .read_all(|packet| {
                    defmt::info!("Received {:?} bytes", packet.len());
                    defmt::info!("Payload {:?}", packet);
                })
                .unwrap();
        }
    }
}
