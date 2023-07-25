#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embedded_alloc::Heap;
use nrf24l01::{TXConfig, PALevel, NRF24L01, OperatingMode, RXConfig, DataRate};
use panic_probe as _;
use defmt_serial as _;

use core::cell::RefCell;

use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_sync::blocking_mutex::{Mutex, raw::NoopRawMutex};
use embassy_executor::Spawner;
use embassy_rp::{
    spi::{Spi, Blocking},
    gpio::{Level, Output, Pin}, uart,
};
use embassy_time::Duration;
use embassy_time::Timer;
use embassy_rp::peripherals::UART0;
use embassy_rp::uart::InterruptHandler;

#[global_allocator]
static HEAP: Heap = Heap::empty();

embassy_rp::bind_interrupts!(struct Irqs {
    UART0_IRQ => InterruptHandler<UART0>;
});

async fn blink_n_times<T>(led: &mut Output<'_, T>, n: u32) where T: Pin {
    for _ in 0..n {
        led.set_high();
        Timer::after(Duration::from_millis(200)).await;
        led.set_low();
        Timer::after(Duration::from_millis(200)).await;
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // peripherals
    let peripherals_config = embassy_rp::config::Config::default();
    let peripherals = embassy_rp::init(peripherals_config);
    // LED
    let mut led_output = Output::new(peripherals.PIN_25, Level::Low);
    // UART0
    let uart0_rx = peripherals.PIN_17; // 22, UART0 RX (white)
    let uart0_tx = peripherals.PIN_16; // 21, UART0 TX (blue)
    let uart0_config = uart::Config::default();
    let uart0 = uart::Uart::new(peripherals.UART0, uart0_tx, uart0_rx, Irqs, peripherals.DMA_CH0, peripherals.DMA_CH1, uart0_config);
    // defmt
    defmt_serial::defmt_serial(uart0);
    // SPI0
    defmt::info!("setting up SPI0");
    let spi0_clk = peripherals.PIN_18; // 24, SPI0 SCK (yellow)
    let spi0_mosi = peripherals.PIN_19; // 25, SPI0 TX (orange)
    let spi0_miso =  peripherals.PIN_20; // 26, SPI0 RX (green)
    let spi0_cs = peripherals.PIN_21; // 27, SPI0 CSn (purple)
    let tx_ce = peripherals.PIN_22; // 29 (brown)
    let mut spi0_config = embassy_rp::spi::Config::default();
    spi0_config.frequency = 1_000_000; // TODO: is this right?
    spi0_config.phase = embassy_rp::spi::Phase::CaptureOnFirstTransition; // TODO: is this right?
    spi0_config.polarity = embassy_rp::spi::Polarity::IdleLow; // TODO: is this right?
    let spi0_cs_output = Output::new(spi0_cs, Level::Low);
    let spi0: Spi<'_, _, Blocking> = Spi::new_blocking(peripherals.SPI0, spi0_clk, spi0_mosi, spi0_miso, spi0_config.clone());
    let spi0_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi0));
    let spi0_device = SpiDeviceWithConfig::new(&spi0_bus, spi0_cs_output, spi0_config);
    defmt::info!("set up SPI0");
    // SPI1
    defmt::info!("setting up SPI1");
    let rx_ce = peripherals.PIN_9; // 12 (brown)
    let spi1_clk = peripherals.PIN_10; // 14, SPI1 SCK (yellow)
    let spi1_mosi = peripherals.PIN_11; // 15, SPI1 TX (orange)
    let spi1_miso =  peripherals.PIN_12; // 16, SPI1 RX (green)
    let spi1_cs = peripherals.PIN_13; // 17, SPI1 CSn (purple)
    let mut spi1_config = embassy_rp::spi::Config::default();
    spi1_config.frequency = 1_000_000; // TODO: is this right?
    spi1_config.phase = embassy_rp::spi::Phase::CaptureOnFirstTransition; // TODO: is this right?
    spi1_config.polarity = embassy_rp::spi::Polarity::IdleLow; // TODO: is this right?
    let spi1_cs_output = Output::new(spi1_cs, Level::Low);
    let spi1: Spi<'_, _, Blocking> = Spi::new_blocking(peripherals.SPI1, spi1_clk, spi1_mosi, spi1_miso, spi1_config.clone());
    let spi1_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi1));
    let spi1_device = SpiDeviceWithConfig::new(&spi1_bus, spi1_cs_output, spi1_config);
    defmt::info!("set up SPI1");
    // NRF24L01P transmitter (SPI0)
    defmt::info!("setting up NRF24L01P transmitter");
    let tx_ce_output = Output::new(tx_ce, Level::Low);
    let tx_config = TXConfig {
        data_rate: DataRate::R1Mbps,
        channel: 108,
        pa_level: PALevel::Low,
        pipe0_address: *b"abcde",
        max_retries: 3,
        retry_delay: 2,
        ..Default::default()
    };
    let mut nrf24l01_tx_device = NRF24L01::new(spi1_device, tx_ce_output).unwrap();
    defmt::info!("set up NRF24L01P transmitter");
    // NRF24L01P receiver (SPI1)
    defmt::info!("setting up NRF24L01P receiver");
    let rx_ce_output = Output::new(rx_ce, Level::Low);
    let rx_config = RXConfig {
        data_rate: DataRate::R1Mbps,
        channel: 108,
        pa_level: PALevel::Low,
        pipe0_address: *b"abcde",
        ..Default::default()
    };
    let mut nrf24l01_rx_device = NRF24L01::new(spi0_device, rx_ce_output).unwrap();
    defmt::info!("set up NRF24L01P receiver");
    // set up tx
    defmt::info!("setting up TX");
    nrf24l01_tx_device.configure(&OperatingMode::TX(tx_config)).unwrap();
    nrf24l01_tx_device.flush_output().unwrap();
    defmt::info!("set up TX");
    // set up rx
    defmt::info!("setting up RX");
    nrf24l01_rx_device.configure(&OperatingMode::RX(rx_config)).unwrap();
    nrf24l01_rx_device.flush_input().unwrap();
    nrf24l01_rx_device.listen().unwrap();
    defmt::info!("set up RX");
    // message
    let message = b"sendtest";
    loop {
        // sleep
        Timer::after(Duration::from_millis(1000)).await;
        // transmit
        defmt::info!("transmit: pushing");
        nrf24l01_tx_device.push(0, message).unwrap();
        defmt::info!("transmit: pushed");
        defmt::info!("transmit: sending");
        match nrf24l01_tx_device.send() {
            Ok(n) => {
                defmt::info!("transmit: sent after {} retries", n);
            },
            Err(nrf24l01::ErrorKind::TimeoutError) => {
                defmt::info!("transmit: timed out?");
                defmt::info!("transmit: flushing due to timeout");
                nrf24l01_tx_device.flush_output().unwrap();
                defmt::info!("transmit: flushed due to timeout");
            },
            Err(e) => defmt::panic!("{:?}", e)
        }
        defmt::info!("transmit: sent");    
        // receive
        let data_available = nrf24l01_rx_device.data_available().unwrap();
        defmt::info!("data_available = {}", data_available);
        if data_available {
            nrf24l01_rx_device.read_all(|packet| {
                defmt::info!("Received {:?} bytes", packet.len());
                defmt::info!("Payload {:?}", packet);
            }).unwrap();
        }
    }
}
