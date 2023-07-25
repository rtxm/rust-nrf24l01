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
            output,
            value: Level::Low,
            saved_value: Level::Low,
        })
    }

    pub fn up(&mut self) -> Result<()> {
        match self.output.set_high() {
            Ok(()) => {

            },
            Err(_err) => {
                return Err(anyhow::anyhow!("Error occured!")).map_err(anyhow::Error::msg); // TODO: convert error?
            }
        }
        self.value = Level::High;
        Ok(())
    }

    pub fn down(&mut self) -> Result<()> {
        match self.output.set_low() {
            Ok(()) => {

            },
            Err(_err) => {
                return Err(anyhow::anyhow!("Error occured!")).map_err(anyhow::Error::msg); // TODO: convert error?
            }
        }
        self.value = Level::Low;
        Ok(())
    }

    pub fn save_state(&mut self) -> () {
        self.saved_value = self.value;
    }

    pub fn restore_state(&mut self) -> Result<()> {
        match self.saved_value {
            Level::High => {
                match self.output.set_high() {
                    Ok(()) => {
        
                    },
                    Err(_err) => {
                        return Err(anyhow::anyhow!("Error occured!")).map_err(anyhow::Error::msg); // TODO: convert error?
                    }
                }
            }
            Level::Low => {
                match self.output.set_low() {
                    Ok(()) => {
        
                    },
                    Err(_err) => {
                        return Err(anyhow::anyhow!("Error occured!")).map_err(anyhow::Error::msg); // TODO: convert error?
                    }
                }
            }
        }
        self.value = self.saved_value;
        Ok(())
    }
}
