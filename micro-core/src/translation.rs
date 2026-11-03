//! Таблица преобразования адресов (дешифратор кода операции): 20 строк.
//!
//! Каждая строка: начальный адрес микропрограммы + 16-символьный шаблон кода
//! операции, где `0`/`1` — фиксированные биты, `X` — любой бит.

pub const TABLE_ROWS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow {
    pub address: u8,
    /// 16 символов: '0' | '1' | 'X' (регистр символа — верхний, как в файлах).
    pub opcode: String,
}

impl Default for TableRow {
    fn default() -> Self {
        Self {
            address: 0,
            opcode: "0000000000000000".to_string(),
        }
    }
}

impl TableRow {
    pub fn new(address: u8, opcode: impl Into<String>) -> Self {
        Self {
            address,
            opcode: opcode.into(),
        }
    }

    /// Совпадает ли шаблон с кодом операции RGK.
    pub fn matches(&self, rgk: u16) -> bool {
        let opcode = self.opcode.as_bytes();
        if opcode.len() != 16 {
            return false;
        }
        // Бит 15 RGK соответствует первому символу шаблона.
        for (i, &c) in opcode.iter().enumerate() {
            let bit = (rgk >> (15 - i)) & 1;
            match c {
                b'0' => {
                    if bit != 0 {
                        return false;
                    }
                }
                b'1'
                    if bit != 1 => {
                        return false;
                    }
                _ => {}
            }
        }
        true
    }
}

#[derive(Debug, Clone)]
pub struct AddressTranslationTable {
    pub rows: Vec<TableRow>,
}

impl Default for AddressTranslationTable {
    fn default() -> Self {
        Self {
            rows: (0..TABLE_ROWS).map(|_| TableRow::default()).collect(),
        }
    }
}

impl AddressTranslationTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Дешифрация RGK: первая строка, чей шаблон совпадает.
    /// Возвращает адрес микропрограммы или `None`.
    pub fn decode(&self, rgk: u16) -> Option<u8> {
        self.rows.iter().find(|r| r.matches(rgk)).map(|r| r.address)
    }

    pub fn reset(&mut self) {
        self.rows = (0..TABLE_ROWS).map(|_| TableRow::default()).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_matching() {
        let row = TableRow::new(0x10, "010XX00100000000");
        // X на позициях p3 (б12), p4 (б11) — допустимы любые комбинации
        assert!(row.matches(0b0100_0001_0000_0000));
        assert!(row.matches(0b0101_0001_0000_0000));
        assert!(row.matches(0b0100_1001_0000_0000));
        assert!(row.matches(0b0101_1001_0000_0000));
        // Несовпадения по фиксированным битам
        assert!(!row.matches(0b1100_0001_0000_0000)); // p0
        assert!(!row.matches(0b0000_0001_0000_0000)); // p1
        assert!(!row.matches(0b0110_0001_0000_0000)); // p2
        assert!(!row.matches(0b0100_0101_0000_0000)); // p5 (б10)
        assert!(!row.matches(0b0100_0011_0000_0000)); // p6 (б9)
        assert!(!row.matches(0b0100_0000_0000_0000)); // p7 (б8)
    }

    #[test]
    fn decode_first_match_wins() {
        let mut table = AddressTranslationTable::new();
        // Заполняем все строки непревпадающим шаблоном, чтобы декодирование
        // зависело только от строк 0 и 1
        for row in table.rows.iter_mut() {
            *row = TableRow::new(0xFF, "1111111111111111");
        }
        table.rows[0] = TableRow::new(0x20, "1000000000000000");
        table.rows[1] = TableRow::new(0x30, "1000X00000000000");
        assert_eq!(table.decode(0x8000), Some(0x20));
        assert_eq!(table.decode(0x8800), Some(0x30));
        assert_eq!(table.decode(0x0000), None);
    }
}
