use crate::command::{FlushRx, FlushTx, Nop};
use crate::registers::{Status, RfCh, RfSetup, TxAddr, EnRxaddr, SetupRetr, EnAa, SetupAw, Dynpd, Feature};
use crate::device::Device;
use crate::PIPES_COUNT;

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

    fn flush_rx(&mut self) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        self.device()
            .send_command(&FlushRx)?;
        Ok(())
    }

    fn flush_tx(&mut self) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        self.device()
            .send_command(&FlushTx)?;
        Ok(())
    }

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

    /// power: `0`: -18 dBm, `3`: 0 dBm
    fn set_rf(&mut self, rate: DataRate, power: u8) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        assert!(power < 0b100);
        let mut register = RfSetup(0);
        register.set_rf_pwr(power);

        let (dr_low, dr_high) = match rate {
            DataRate::R250Kbps => (true, false),
            DataRate::R1Mbps => (false, false),
            DataRate::R2Mbps => (false, true),
        };
        register.set_rf_dr_low(dr_low);
        register.set_rf_dr_high(dr_high);

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

    fn set_pipes_rx_enable(&mut self, bools: &[bool; PIPES_COUNT]) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        self.device()
            .write_register(EnRxaddr::from_bools(bools))?;
        Ok(())
    }

    fn set_rx_addr(&mut self, pipe_no: usize, addr: &[u8]) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        macro_rules! w {
            ( $($no: expr, $name: ident);+ ) => (
                match pipe_no {
                    $(
                        $no => {
                            use crate::registers::$name;
                            let register = $name::new(addr);
                            self.device().write_register(register)?;
                        }
                    )+
                        _ => panic!("No such pipe {}", pipe_no)
                }
            )
        }
        w!(0, RxAddrP0;
           1, RxAddrP1;
           2, RxAddrP2;
           3, RxAddrP3;
           4, RxAddrP4;
           5, RxAddrP5);
        Ok(())
    }

    fn set_tx_addr(&mut self, addr: &[u8]) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        let register = TxAddr::new(addr);
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

    fn get_auto_ack(&mut self) -> Result<[bool; PIPES_COUNT], <<Self as Configuration>::Inner as Device>::Error> {
        // Read
        let (_, register) = self.device().read_register::<EnAa>()?;
        Ok(register.to_bools())
    }

    fn set_auto_ack(&mut self, bools: &[bool; PIPES_COUNT]) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        // Convert back
        let register = EnAa::from_bools(bools);
        // Write back
        self.device()
            .write_register(register)?;
        Ok(())
    }

    fn get_address_width(&mut self) -> Result<u8, <<Self as Configuration>::Inner as Device>::Error> {
        let (_, register) =
            self.device()
            .read_register::<SetupAw>()?;
        Ok(2 + register.aw())
    }

    fn get_interrupts(&mut self) -> Result<(bool, bool, bool), <<Self as Configuration>::Inner as Device>::Error> {
        let (status, ()) = self.device()
            .send_command(&Nop)?;
        Ok((
            status.rx_dr(),
            status.tx_ds(),
            status.max_rt()
        ))
    }

    fn clear_interrupts(&mut self) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        let mut clear = Status(0);
        clear.set_rx_dr(true);
        clear.set_tx_ds(true);
        clear.set_max_rt(true);
        self.device()
            .write_register(clear)?;
        Ok(())
    }

    /// ## `bools`
    /// * `None`: Dynamic payload length
    /// * `Some(len)`: Static payload length `len`
    fn set_pipes_rx_lengths(
        &mut self,
        lengths: &[Option<u8>; PIPES_COUNT]
    ) -> Result<(), <<Self as Configuration>::Inner as Device>::Error> {
        // Enable dynamic payload lengths
        let mut bools = [true; PIPES_COUNT];
        for (i, length) in lengths.iter().enumerate() {
            bools[i] = length.is_none();
        }
        let dynpd = Dynpd::from_bools(&bools);
        if dynpd.0 != 0 {
            self.device()
                .update_register::<Feature, _, _>(|feature| {
                    feature.set_en_dpl(true);
                })?;
        }
        self.device()
            .write_register(dynpd)?;

        // Set static payload lengths
        macro_rules! set_rx_pw {
            ($name: ident, $index: expr) => ({
                use crate::registers::$name;
                let length = lengths[$index]
                    .unwrap_or(0);
                let mut register = $name(0);
                register.set(length);
                self.device()
                    .write_register(register)?;
            })
        }
        set_rx_pw!(RxPwP0, 0);
        set_rx_pw!(RxPwP1, 1);
        set_rx_pw!(RxPwP2, 2);
        set_rx_pw!(RxPwP3, 3);
        set_rx_pw!(RxPwP4, 4);
        set_rx_pw!(RxPwP5, 5);

        Ok(())
    }
}
