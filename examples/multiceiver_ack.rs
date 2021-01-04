extern crate nrf24l01;

use std::thread::sleep;
use std::time::Duration;

use nrf24l01::{OperatingMode, PALevel, RXConfig, NRF24L01};

fn main() {
    let config = RXConfig {
        channel: 108,
        pa_level: PALevel::Low,
        pipe0_address: *b"0node",
        pipe1_address: Some(*b"1node"),
        pipe2_addr_lsb: Some(b'2'),
        pipe3_addr_lsb: Some(b'3'),
        ..Default::default()
    };
    let mut device = NRF24L01::new(25, 0).unwrap();
    device.configure(&OperatingMode::RX(config)).unwrap();
    device.flush_output().unwrap();
    device.flush_input().unwrap();
    device.listen().unwrap();
    // Prepare ack payloads for next receptions.
    // Each payload will be transmitted as the ACK response
    // to the first packet that arrives on its respective pipe.
    // Remember: a payload sent as ACK for a packet on pipe P remains in the
    // output FIFO until pipe P receives a new, *different*, packet.
    device.push(0, b"ack payload for node0").unwrap();
    device.push(0, b"ack payload for node0 bis").unwrap();
    device.push(2, b"ack payload for node2").unwrap();
    loop {
        sleep(Duration::from_millis(500));
        if device.data_available().unwrap() {
            device
                .read_all(|packet| {
                    println!("Received {:?} bytes", packet.len());
                    println!("Payload {:?}", packet);
                })
                .unwrap();
        }
    }
}
