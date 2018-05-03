use core::fmt;
use command::WriteTxPayload;
use registers::{FifoStatus, ObserveTx};
use device::Device;
use standby::StandbyMode;
use payload::Payload;
use config::Configuration;

use core::fmt::Write;
use cortex_m_semihosting::hio;

/// Represents **TX Mode** and the associated **TX Settling** and
/// **Standby-II** states.
pub struct TxMode<D: Device> {
    device: D,
}

impl<D: Device> fmt::Debug for TxMode<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TxMode")
    }
}

impl<D: Device> TxMode<D> {
    /// Relies on everything being set up by `StandbyMode::tx()`, from
    /// which it is called
    pub(crate) fn new(device: D) -> Self {
        TxMode { device }
    }

    /// Disable `CE` so that you can switch into RX mode.
    pub fn standby(mut self) -> Result<StandbyMode<D>, D::Error> {
        self.wait_empty()?;

        Ok(StandbyMode::from_rx_tx(self.device))
    }

    /// Is TX FIFO empty?
    pub fn is_empty(&mut self) -> Result<bool, D::Error> {
        let (status, fifo_status) =
            self.device.read_register::<FifoStatus>()?;
        Ok(fifo_status.tx_empty())
    }

    /// Is TX FIFO full?
    pub fn is_full(&mut self) -> Result<bool, D::Error> {
        let (_, fifo_status) =
            self.device.read_register::<FifoStatus>()?;
        Ok(fifo_status.tx_full())
    }

    /// Does the TX FIFO have space?
    pub fn can_send(&mut self) -> Result<bool, D::Error> {
        self.is_full()
            .map(|full| !full)
    }

    /// Send asynchronously
    pub fn send(&mut self, packet: &[u8]) -> Result<(), D::Error> {
        self.device.send_command(&WriteTxPayload::new(packet))?;
        self.device.ce_enable();
        Ok(())
    }

    /// Wait until FX FIFO is empty
    pub fn wait_empty(&mut self) -> Result<(), D::Error> {
        if ! self.is_empty()? {
            self.device.ce_enable();
            let mut n = 0usize;
            while n < 5 && ! self.is_empty()?  {
                n += 1;
                let o = self.observe()?;
                let mut stdout = hio::hstdout().unwrap();
                writeln!(stdout, "plos={:X}\tarc={:X}", o.plos_cnt(), o.arc_cnt());
            }
            self.device.ce_disable();
        }
        // let mut stdout = hio::hstdout().unwrap();
        // writeln!(stdout, "TX is empty!");

        Ok(())
    }

    pub fn observe(&mut self) -> Result<ObserveTx, D::Error> {
        let (_, observe_tx) =
            self.device.read_register()?;
        Ok(observe_tx)
    }
}

impl<D: Device> Configuration for TxMode<D> {
    type Inner = D;
    fn device(&mut self) -> &mut Self::Inner {
        &mut self.device
    }
}
