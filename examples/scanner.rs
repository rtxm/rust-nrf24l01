extern crate nrf24l01;

use std::env;

use nrf24l01::NRF24L01;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 5 {
        let ce: u64 = match args[1].parse() {
            Ok(n) => n,
            _ => {
                println!("<ce> must be an interger!");
                return;
            }
        };
        let spi: u8 = match args[2].parse() {
            Ok(n) => n,
            _ => {
                println!("<spi> must be an integer!");
                return;
            }
        };
        let rounds: u32 = match args[3].parse() {
            Ok(n) => n,
            _ => {
                println!("<rounds> must be an integer!");
                return;
            }
        };
        let delay: u32 = match args[4].parse() {
            Ok(n) => n,
            _ => {
                println!("<delay> must be an integer!");
                return;
            }
        };
        let mut table = [0u32; 126];
        let mut nrf = NRF24L01::new(ce, spi).unwrap();
        nrf.scan(rounds, delay, &mut table).unwrap();
        // for channel in 0..126 {
        //     println!("Channel {:}: {:}", channel, table[channel]);
        // }
        for (channel, obs) in table.iter().enumerate().take(126) {
            println!("Channel {:}: {:}", channel, obs);
        }
    } else {
        println!("Usage scanner <ce> <spi> <rounds> <delay>");
    }
}
