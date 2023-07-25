use embassy_rp::gpio::Level;
use embedded_hal::digital::OutputPin;
use embedded_hal::digital::ErrorKind;

pub struct CEPin<T> {
    output: T,
    value: Level,
    saved_value: Level,
}

impl<T> CEPin<T> where T: OutputPin {
    // add code here
    pub fn new(output: T) -> Result<CEPin<T>, ErrorKind> {
        Ok(CEPin {
            output,
            value: Level::Low,
            saved_value: Level::Low,
        })
    }

    pub fn up(&mut self) -> Result<(), ErrorKind> {
        self.output.set_high().map_err(|_| ErrorKind::Other)?;
        self.value = Level::High;
        Ok(())
    }

    pub fn down(&mut self) -> Result<(), ErrorKind> {
        self.output.set_low().map_err(|_| ErrorKind::Other)?;
        self.value = Level::Low;
        Ok(())
    }

    pub fn save_state(&mut self) -> () {
        self.saved_value = self.value;
    }

    pub fn restore_state(&mut self) -> Result<(), ErrorKind> {
        match self.saved_value {
            Level::High => {
                self.output.set_high().map_err(|_| ErrorKind::Other)?;
            }
            Level::Low => {
                self.output.set_low().map_err(|_| ErrorKind::Other)?;
            }
        }
        self.value = self.saved_value;
        Ok(())
    }
}
