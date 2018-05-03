pub trait Register {
    /// Address in the register map
    fn addr() -> u8;

    fn data_bytes() -> usize;
    fn encode(&self, &mut [u8]);
    fn decode(&[u8]) -> Self;
}

/// Common for all registers with 1 bytes of data
macro_rules! impl_register {
    ($name: ident, $addr: expr) => (
        impl Register for $name {
            fn addr() -> u8 {
                $addr
            }

            fn data_bytes() -> usize {
                1
            }

            fn encode(&self, buf: &mut [u8]) {
                buf[0] = self.0;
            }

            fn decode(buf: &[u8]) -> Self {
                $name(buf[0])
            }
        }

        impl Clone for $name {
            fn clone(&self) -> Self {
                $name(self.0)
            }
        }

        impl PartialEq for $name {
            fn eq(&self, rhs: &Self) -> bool {
                self.0 == rhs.0
            }
        }
    )
}

macro_rules! impl_buffered_register {
    ($name: ident, $addr: expr, $size: expr) => (
        impl Register for $name {
            fn addr() -> u8 {
                $addr
            }

            fn data_bytes() -> usize {
                $size
            }

            fn encode(&self, buf: &mut [u8]) {
                buf.copy_from_slice(&self.0);
            }

            fn decode(buf: &[u8]) -> Self {
                let mut addr = [0; $size];
                addr.copy_from_slice(buf);
                $name(addr)
            }
        }
    )
}

bitfield! {
    pub struct Config(u8);
    impl Debug;

    /// Mask interrupt
    pub mask_rx_dr, set_mask_rx_dr: 6;
    /// Mask interrupt
    pub mask_tx_ds, set_mask_tx_ds: 5;
    /// Mask interrupt
    pub mask_max_rt, set_mask_max_rt: 4;
    /// Enable CRC
    pub en_crc, set_en_crc: 3;
    /// CRC encoding scheme
    /// * `0`: 1 byte
    /// * `1`: 2 bytes
    pub crco, set_crco: 2;
    /// Power up
    pub pwr_up, set_pwr_up: 1;
    /// RX/TX control
    /// * `1`: PRX
    /// * `0`: PTX
    pub prim_rx, set_prim_rx: 0;
}
impl_register!(Config, 0x00);

/// Enable Auto Acknowledgment
pub struct EnAa(pub u8);
impl_register!(EnAa, 0x01);

impl EnAa {
    pub fn enaa_p(&self, pipe_no: usize) -> bool {
        let mask = 1 << pipe_no;
        self.0 & mask == mask
    }

    pub fn set_enaa_p(&mut self, pipe_no: usize, enable: bool) {
        let mask = 1 << pipe_no;
        if enable {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }

    pub fn from_bools(bools: &[bool; 6]) -> Self {
        let mut register = EnAa(0b0011_1111);
        for (i, b) in bools.iter().enumerate() {
            register.set_enaa_p(i, *b);
        }
        register
    }
    
    pub fn to_bools(&self) -> [bool; 6] {
        let mut bools = [true; 6];
        for (i, b) in bools.iter_mut().enumerate() {
            *b = self.enaa_p(i);
        }
        bools
    }
}

#[derive(Debug)]
pub struct EnRxaddr(u8);
impl_register!(EnRxaddr, 0x02);

/// Enabled RX Addresses
impl EnRxaddr {
    pub fn erx_p(&self, pipe_no: u8) -> bool {
        let mask = 1 << pipe_no;
        self.0 & mask == mask
    }

    pub fn set_erx_p(&mut self, pipe_no: u8, enable: bool) {
        let mask = 1 << pipe_no;
        if enable {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }
}

bitfield! {
    pub struct SetupAw(u8);
    impl Debug;

    /// RX/TX address field width:
    /// * `0b01`: 3 bytes
    /// * `0b10`: 4 bytes
    /// * `0b11`: 5 bytes
    pub u8, aw, set_aw: 1, 0;
}
impl_register!(SetupAw, 0x03);

bitfield! {
    /// Setup of Automatic Retransmission
    pub struct SetupRetr(u8);
    impl Debug;

    /// Auto Retransmit Delay, where the actualy delay is `250 + (250 * ard) µS`
    pub u8, ard, set_ard: 7, 4;
    /// Auto Retransmit Count
    pub u8, arc, set_arc: 3, 0;
}
impl_register!(SetupRetr, 0x04);

bitfield! {
    /// RF Channel
    pub struct RfCh(u8);
    impl Debug;

    /// Frequency, that is `2400 + rf_ch` Mhz
    pub u8, rf_ch, set_rf_ch: 6, 0;
}
impl_register!(RfCh, 0x05);

bitfield! {
    /// RF Setup
    pub struct RfSetup(u8);
    impl Debug;

    /// Set for 250 kbps
    pub rf_dr_low, set_rf_dr_low: 5;
    /// Set for 2 Mbps
    pub rf_dr_high, set_rf_dr_high: 3;
    /// RF output power in TX mode
    /// * `00`: -18 dBm
    /// * `01`: -12 dBm
    /// * `10`: -6 dBm
    /// * `11`: 0 dBm
    pub u8, rf_pwr, set_rf_pwr: 2, 1;
}
impl_register!(RfSetup, 0x06);

bitfield! {
    /// Status register, always received on MISO while command is sent
    /// on MOSI.
    pub struct Status(u8);
    impl Debug;

    /// Data ready RX FIFO interrupt. Write `true` to clear.
    pub rx_dr, set_rx_dr: 6;
    /// Data sent TX FIFO interrupt. Write `true` to clear.
    pub tx_ds, set_tx_ds: 5;
    /// Maximum number of TX retransmits interrupt. Write `true` to clear.
    pub max_rt, set_max_rt: 4;
    /// Data pipe number for reading from RX FIFO
    pub u8, rx_p_no, _: 3, 1;
    /// TX FIFO full flag
    pub tx_full, _: 0;
}
impl_register!(Status, 0x07);

bitfield! {
    pub struct ObserveTx(u8);
    impl Debug;

    pub u8, plos_cnt, _: 7, 4;
    pub u8, arc_cnt, _: 3, 0;
}
impl_register!(ObserveTx, 0x08);

pub struct RxAddrP0(pub [u8; 5]);
impl_buffered_register!(RxAddrP0, 0x0A, 5);

pub struct TxAddr(pub [u8; 5]);
impl_buffered_register!(TxAddr, 0x10, 5);

bitfield! {
    /// Status register, always received on MISO while command is sent
    /// on MOSI.
    pub struct FifoStatus(u8);
    impl Debug;

    pub tx_reuse, _: 6;
    /// TX FIFO full flag
    pub tx_full, _: 5;
    /// TX FIFO empty flag
    pub tx_empty, _: 4;
    /// RX FIFO full flag
    pub rx_full, _: 1;
    /// RX FIFO empty flag
    pub rx_empty, _: 0;
}
impl_register!(FifoStatus, 0x17);

