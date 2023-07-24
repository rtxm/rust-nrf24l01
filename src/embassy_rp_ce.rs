use anyhow::Result;
use embassy_rp::gpio::Level;
use embedded_hal::digital::OutputPin;

pub struct CEPin<T> {
    output: T,
    value: Level,
    saved_value: Level,
}

impl<T> CEPin<T> where T: OutputPin {
    // add code here
    pub fn new(output: T) -> Result<CEPin<T>> {
        Ok(CEPin {
            output: output,
            value: Level::Low,
            saved_value: Level::Low,
        })
    }

    pub fn up(&mut self) -> Result<()> {
        self.output.set_high().unwrap(); // TODO: error checking?
        self.value = Level::High;
        Ok(())
    }

    pub fn down(&mut self) -> Result<()> {
        self.output.set_low().unwrap(); // TODO: error checking?
        self.value = Level::Low;
        Ok(())
    }

    pub fn save_state(&mut self) -> () {
        self.saved_value = self.value;
    }

    pub fn restore_state(&mut self) -> Result<()> {
        match self.saved_value {
            Level::High => {
                self.output.set_high().unwrap(); // TODO: error checking?
            }
            Level::Low => {
                self.output.set_low().unwrap(); // TODO: error checking?
            }
        }
        self.value = self.saved_value;
        Ok(())
    }
}
