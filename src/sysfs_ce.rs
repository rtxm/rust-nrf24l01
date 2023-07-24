extern crate sysfs_gpio;

use std::io;

pub struct CEPin {
    ce_pin: sysfs_gpio::Pin,
    value: u8,
    saved_value: u8,
}

impl CEPin {
    // add code here
    pub fn new(pin_num: u64) -> Result<CEPin> {
        let ce = sysfs_gpio::Pin::new(pin_num);
        ce.export().or_else(|_| {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Unable to export CE",
            ))
        })?;
        ce.set_direction(sysfs_gpio::Direction::Low).or_else(|_| {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Unable to set CE",
            ))
        })?;
        Ok(CEPin {
            ce_pin: ce,
            value: 0,
            saved_value: 0,
        })
    }

    pub fn up(&mut self) -> Result<()> {
        self.ce_pin.set_value(1).unwrap();
        self.value = 1;
        Ok(())
    }

    pub fn down(&mut self) -> Result<()> {
        self.ce_pin.set_value(0).unwrap();
        self.value = 0;
        Ok(())
    }

    pub fn save_state(&mut self) -> () {
        self.saved_value = self.value;
    }

    pub fn restore_state(&mut self) -> Result<()> {
        self.ce_pin.set_value(self.saved_value).unwrap();
        self.value = self.saved_value;
        Ok(())
    }
}
