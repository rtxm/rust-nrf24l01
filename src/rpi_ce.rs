extern crate rppal;

use std::io;

use self::rppal::gpio::{Level, Mode, GPIO};

pub struct CEPin {
    gpio: GPIO,
    ce_pin: u8,
    value: Level,
    saved_value: Level,
}

impl CEPin {
    // add code here
    pub fn new(pin_num: u64) -> io::Result<CEPin> {
        let pin_num8 = pin_num as u8;
        let mut gpio = GPIO::new().unwrap();
        gpio.set_mode(pin_num8, Mode::Output);
        Ok(CEPin {
            gpio: gpio,
            ce_pin: pin_num8,
            value: Level::Low,
            saved_value: Level::Low,
        })
    }

    pub fn up(&mut self) -> io::Result<()> {
        self.gpio.write(self.ce_pin, Level::High);
        self.value = Level::High;
        Ok(())
    }

    pub fn down(&mut self) -> io::Result<()> {
        self.gpio.write(self.ce_pin, Level::Low);
        self.value = Level::Low;
        Ok(())
    }

    pub fn save_state(&mut self) -> () {
        self.saved_value = self.value;
    }

    pub fn restore_state(&mut self) -> io::Result<()> {
        self.gpio.write(self.ce_pin, self.saved_value);
        self.value = self.saved_value;
        Ok(())
    }
}
