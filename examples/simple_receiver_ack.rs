extern crate nrf24l01;

use std::time::Duration;
use std::thread::sleep;

use nrf24l01::{RXConfig, NRF24L01, PALevel, OperatingMode};

fn main() {
    let config = RXConfig {
        channel: 108,
        pa_level: PALevel::Low,
        pipe0_address: *b"abcde",
        ..Default::default()
    };
    let mut device = NRF24L01::new(25, 0).unwrap();
    device.configure(&OperatingMode::RX(config)).unwrap();
    device.listen().unwrap();
    loop {
        sleep(Duration::from_millis(500));
        if device.data_available().unwrap() {
            device
                .read_all(|packet| {
                    println!("Received {:?} bytes", packet.len());
                    println!("Payload {:?}", packet);
                })
                .unwrap();
            // prepare ack payload for next reception
            device.push(0, b"ack payload").unwrap();
        }
    }
}
