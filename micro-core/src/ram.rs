//! Оперативная память: 1024 байта, побайтовый и пословный доступ (little-endian).

pub const RAM_SIZE: usize = 1024;

#[derive(Debug, Clone)]
pub struct RamMemory {
    data: Vec<u8>,
    /// Маска байтов, изменённых последней МК (для подсветки в UI).
    changed: Vec<bool>,
}

impl Default for RamMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl RamMemory {
    pub fn new() -> Self {
        Self {
            data: vec![0u8; RAM_SIZE],
            changed: vec![false; RAM_SIZE],
        }
    }

    pub fn size(&self) -> usize {
        RAM_SIZE
    }

    #[inline]
    pub fn get(&self, address: u16) -> u8 {
        self.data[address as usize]
    }

    #[inline]
    pub fn set(&mut self, address: u16, value: u8) {
        let i = address as usize;
        if self.data[i] != value {
            self.data[i] = value;
        }
        self.changed[i] = true;
    }

    /// Слово (little-endian) по адресу. `None`, если пара байт выходит за границы.
    pub fn get_word(&self, address: u16) -> Option<u16> {
        let i = address as usize;
        if i + 1 < RAM_SIZE {
            Some((self.data[i] as u16) | ((self.data[i + 1] as u16) << 8))
        } else {
            None
        }
    }

    pub fn set_word(&mut self, address: u16, value: u16) -> bool {
        let i = address as usize;
        if i + 1 < RAM_SIZE {
            self.data[i] = (value & 0xFF) as u8;
            self.data[i + 1] = (value >> 8) as u8;
            self.changed[i] = true;
            self.changed[i + 1] = true;
            true
        } else {
            false
        }
    }

    /// Изменён ли байт с момента `take_changes`.
    #[inline]
    pub fn is_changed(&self, address: u16) -> bool {
        self.changed[address as usize]
    }

    pub fn take_changes(&mut self) {
        self.changed.iter_mut().for_each(|c| *c = false);
    }

    /// Прямая запись (загрузка файла, редактирование): без отметки об изменении.
    pub fn load_bytes(&mut self, bytes: &[u8]) {
        let n = bytes.len().min(RAM_SIZE);
        self.data[..n].copy_from_slice(&bytes[..n]);
    }

    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn clear(&mut self) {
        self.data.iter_mut().for_each(|b| *b = 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_little_endian() {
        let mut ram = RamMemory::new();
        ram.set(0x10, 0x34);
        ram.set(0x11, 0x12);
        assert_eq!(ram.get_word(0x10), Some(0x1234));
        ram.set_word(0x20, 0xABCD);
        assert_eq!(ram.get(0x20), 0xCD);
        assert_eq!(ram.get(0x21), 0xAB);
    }

    #[test]
    fn word_at_end_fails() {
        let ram = RamMemory::new();
        assert!(ram.get_word(RAM_SIZE as u16 - 1).is_none());
        assert!(ram.get_word(RAM_SIZE as u16 - 2).is_some());
    }
}
