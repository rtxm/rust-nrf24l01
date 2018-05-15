use core::ops::Deref;

pub struct Payload {
    data: [u8; 32],
    len: usize,
}

impl Payload {
    pub fn new(source: &[u8]) -> Self {
        let mut data = [0; 32];
        let len = source.len().min(data.len());
        data[0..len].copy_from_slice(&source[0..len]);
        Payload { data, len }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl AsRef<[u8]> for Payload {
    fn as_ref(&self) -> &[u8] {
        &self.data[0..self.len]
    }
}

impl Deref for Payload {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.as_ref()
    }
}
