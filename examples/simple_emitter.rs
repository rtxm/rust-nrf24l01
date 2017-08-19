extern crate nrf24l01;

use std::time::Duration;
use std::thread::sleep;

use nrf24l01::{TXConfig, NRF24L01, PALevel, OperatingMode};

fn main() {
    let config = TXConfig {
        channel: 108,
        pa_level: PALevel::Low,
        pipe0_address: *b"abcde",
        max_retries: 3,
        retry_delay: 2,
        ..Default::default()
    };
    let mut device = NRF24L01::new(25, 0).unwrap();
    let message = b"sendtest";
    device.configure(&OperatingMode::TX(config)).unwrap();
    device.flush().unwrap();
    loop {
        device.push(0, message).unwrap();
        match device.send() {
            Ok(retries) => println!("Message sent, {} retries needed", retries),
            Err(err) => {
                println!("Destination unreachable: {:?}", err);
                device.flush().unwrap()
            }
        };
        sleep(Duration::from_millis(5000));
    }
}
