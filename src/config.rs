use registers::{RfCh, TxAddr, SetupRetr};
use device::Device;

/// Supported air data rates.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum DataRate {
    R250Kbps,
    R1Mbps,
    R2Mbps,
}

impl Default for DataRate {
    fn default() -> DataRate {
        DataRate::R1Mbps
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum CrcMode {
    OneByte,
    TwoBytes,
}

pub trait Configuration {
    type Inner: Device;
    fn device(&mut self) -> &mut Self::Inner;

    fn get_frequency(&mut self) -> Result<u8, <<Self as Configuration>::Inner as Device>::Error> {
        let (_, register) =
            self.device().read_register::<RfCh>()?;
        let freq_offset = register.rf_ch();
        Ok(freq_offset)
    }

    fn set_frequency(&mut self, freq_offset: u8) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        assert!(freq_offset < 126);

        let mut register = RfCh(0);
        register.set_rf_ch(freq_offset);
        self.device()
            .write_register(register)?;

        Ok(())
    }

    fn set_crc(&mut self, mode: Option<CrcMode>) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        self.device().update_config(|config| {
            match mode {
                None       => config.set_en_crc(false),
                Some(mode) => match mode {
                    CrcMode::OneByte  => config.set_crco(false),
                    CrcMode::TwoBytes => config.set_crco(true),
                },
            }
        })
    }

    fn set_tx_addr(&mut self, tx_addr: [u8; 5]) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        let register = TxAddr(tx_addr);
        self.device().write_register(register)?;
        Ok(())
    }

    fn set_auto_retransmit(&mut self, delay: u8, count: u8) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        let mut register = SetupRetr(0);
        register.set_ard(delay);
        register.set_arc(count);
        self.device().write_register(register)?;
        Ok(())
    }
}
