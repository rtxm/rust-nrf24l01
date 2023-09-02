# rust-nrf24l01

[![Version](https://img.shields.io/crates/v/nrf24l01.svg)](https://crates.io/crates/nrf24l01)

* [Documentation](https://docs.rs/nrf24l01/0.1.0/)

**Notice: requires Rust <= 1.58**

A pure Rust user space driver for NRF24L01(+) transceivers on Linux.

The aim of this driver is to provide a rustic, easy to use, no non-sense
API to drive an NRF24L01(+) transceiver.

This is not a port from another language, this driver has been written from scratch
based on the device specs.

For the moment, the driver only exposes an API for the most reliable communication
scheme offered by NRF24L01 chips, that is _Enhanced Shockburst_ ™:
automatic (hardware) packet acknowlegement with optional payload, dynamic payload length and
long CRC (2 bytes).

The code has been tested on a Raspberry Pi with success. It should work on any platform supported
by [rust-spidev][1] and [rust-sysfs-gpio][2].

## Usage

Add a dependency to `nrf24l01` to your `Cargo.toml`:

```toml
[dependencies]
nrf24l01 = "0.2.0"
```

## Examples

### Simple emitter

```rust
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
    device.flush_output().unwrap();

    loop {
        device.push(0, message).unwrap();
        match device.send() {
            Ok(retries) => println!("Message sent, {} retries needed", retries),
            Err(err) => {
                println!("Destination unreachable: {:?}", err);
                device.flush_output().unwrap()
            }
        };
        sleep(Duration::from_millis(5000));
    }
}
```

### Simple receiver listening to the simple emitter

```rust
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
        }
    }
}
```

### More examples

* [simple_emitter_ack.rs](https://github.com/rtxm/rust-nrf24l01/blob/master/examples/simple_emitter_ack.rs): simple emitter that tests for and reads ACK payloads;
* [simple_receiver_ack.rs](https://github.com/rtxm/rust-nrf24l01/blob/master/examples/simple_receiver_ack.rs): a simple receiver that attaches payloads to ACKs;
* [multiceiver_ack.rs](https://github.com/rtxm/rust-nrf24l01/blob/master/examples/multiceiver_ack.rs): an example of a multiceiver that attaches distinct payloads for specific peers.

## Cross-compilation

The [rust-cross guide][3] has detailled and comprehensive instructions for cross compiling.

Once you are set up for say, ARM, you can cross-compile the examples for a Raspberry Pi as easily as:

```bash
cargo build -v --examples --target=arm-unknown-linux-gnueabihf
```

Then, you can move any of the executables to your test machine:

```bash
scp target/arm-unknown-linux-gnueabihf/debug/examples/multiceiver_ack ...
```

## Performance

For SPI communication with the NRF24L01(+), we use the Linux standard SPIDEV kernel driver through the [rust-spidev][1] library with excellent efficiency.

For driving the device CE pin, we use GPIO. The current standard Linux way is by using the sysfs-gpio kernel driver. For that we use [rust-sysfs-gpio][2].

Unfortunatly, sysfs-gpio is slow, and rust-sysfs-gpio slower still. On a Raspberry A+ for example, that incurs a ~300 µs lag for each send or read operation
(for comparison, that's roughly the time needed by the device to send 32 bytes and receive acknowledgment).

If you need really fast operation and you use a Raspberry Pi, you can activate the ``rpi_accel`` feature. That feature uses [rppal][4] for direct access to the GPIO registers exposed on Raspberry Pi in ``/dev/mem`` or ``/dev/gpiomem``. Thus the lag becomes negligible: only ~130 ns on a Rasberry Pi A+.

To enable the `rpi_accel` feature for your crate (and disabling the defaults), replace the dependency on this library in your `Cargo.toml` by:

```toml
[dependencies.nrf24l01]
version = "0.2.0"
features = ["rpi_accel"]
default-features = false
```

If you know a library for such a fast access to GPIO on the Beaglebone, please let me know!

## Future

In the future, I'd like to provide :

* asynchronous operation, with mio;
* a stream based API for fast, "real time", data transmission
at the cost of possible packet loss.

I'm still quite new to Rust, so the code may be suboptimal. Feel free to submit pull requests to improve it!


[1]: https://github.com/rust-embedded/rust-spidev
[2]: https://github.com/rust-embedded/rust-sysfs-gpio
[3]: https://github.com/japaric/rust-cross
[4]: https://github.com/golemparts/rppal
