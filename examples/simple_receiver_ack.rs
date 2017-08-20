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
    let mut packet_buffer = [0u8; 32];
    device.configure(&OperatingMode::RX(config)).unwrap();
    device.listen().unwrap();
    loop {
        sleep(Duration::from_millis(500));
        if device.data_available().unwrap() {
            let packet_size = device.read(&mut packet_buffer).unwrap();
            println!("Received {:?} bytes", packet_size);
            println!("Payload {:?}", &packet_buffer[0..packet_size]);
            // prepare ack payload for next reception
            device.push(0, b"ack payload").unwrap();
        }
    }
}
