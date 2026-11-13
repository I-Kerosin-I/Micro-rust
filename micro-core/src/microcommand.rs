//! Микрокоманда: 18 полей + комментарий.
//!
//! Значения по умолчанию соответствуют конструктору `MicroCommand(byte address)`
//! оригинальной C#-версии.

/// Разыменованное поле микрокоманды (для обобщённого редактора и меню вставки).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    A,
    B,
    Ma,
    Mb,
    Mem,
    Src,
    Sh,
    N,
    Alu,
    Ccx,
    F,
    Dst,
    Wm,
    Jfi,
    Cc,
    Cha,
    Const,
}

impl Field {
    /// Все поля в порядке колонок таблицы МПП.
    pub const ALL: [Field; 17] = [
        Field::A,
        Field::B,
        Field::Ma,
        Field::Mb,
        Field::Mem,
        Field::Src,
        Field::Sh,
        Field::N,
        Field::Alu,
        Field::Ccx,
        Field::F,
        Field::Dst,
        Field::Wm,
        Field::Jfi,
        Field::Cc,
        Field::Cha,
        Field::Const,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Field::A => "A",
            Field::B => "B",
            Field::Ma => "MA",
            Field::Mb => "MB",
            Field::Mem => "MEM",
            Field::Src => "SRC",
            Field::Sh => "SH",
            Field::N => "N",
            Field::Alu => "ALU",
            Field::Ccx => "CCX",
            Field::F => "F",
            Field::Dst => "DST",
            Field::Wm => "WM",
            Field::Jfi => "JFI",
            Field::Cc => "CC",
            Field::Cha => "CHA",
            Field::Const => "CONST",
        }
    }

    /// Сколько HEX-цифр печатать при записи в файл.
    pub const fn hex_width(self) -> usize {
        match self {
            Field::Const => 4,
            _ => 1,
        }
    }

    /// Битовая маска поля: значение усекается до него при записи.
    pub const fn mask(self) -> u16 {
        match self {
            Field::Const => 0xFFFF,
            Field::A | Field::B | Field::N | Field::Alu | Field::Sh => 0x0F,
            Field::Ma | Field::Mb | Field::Ccx | Field::Wm => 0x03,
            Field::F => 0x01,
            Field::Mem | Field::Src | Field::Dst | Field::Jfi | Field::Cc | Field::Cha => 0x07,
        }
    }

    /// Валидация ввода: допустим ли код поля (по справочнику `field_options`).
    /// Значение усекается до размера поля, как при записи; поля без
    /// справочника (N, CONST) принимают любые значения.
    pub fn input_valid(self, value: u16) -> bool {
        if matches!(self, Field::N | Field::Const) {
            return true;
        }
        let v = value & self.mask();
        field_options(self)
            .iter()
            .any(|(allowed, _)| *allowed == v)
    }
}

/// Микрокоманда.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicroCommand {
    pub address: u8,
    pub a: u8,
    pub b: u8,
    pub ma: u8,
    pub mb: u8,
    pub mem: u8,
    pub src: u8,
    pub sh: u8,
    pub n: u8,
    pub alu: u8,
    pub ccx: u8,
    pub f: u8,
    pub dst: u8,
    pub wm: u8,
    pub jfi: u8,
    pub cc: u8,
    pub cha: u8,
    pub const_: u16,
    pub comment: String,
}

impl Default for MicroCommand {
    fn default() -> Self {
        Self::new(0)
    }
}

impl MicroCommand {
    /// Новая МК со значениями по умолчанию (как в оригинале):
    /// SRC=1, ALU=6, CHA=7, остальные нули.
    pub fn new(address: u8) -> Self {
        Self {
            address,
            a: 0,
            b: 0,
            ma: 0,
            mb: 0,
            mem: 0,
            src: 1,
            sh: 0,
            n: 0,
            alu: 6,
            ccx: 0,
            f: 0,
            dst: 0,
            wm: 0,
            jfi: 0,
            cc: 0,
            cha: 7,
            const_: 0,
            comment: String::new(),
        }
    }

    /// Чтение поля по идентификатору.
    pub fn get(&self, field: Field) -> u16 {
        match field {
            Field::A => self.a as u16,
            Field::B => self.b as u16,
            Field::Ma => self.ma as u16,
            Field::Mb => self.mb as u16,
            Field::Mem => self.mem as u16,
            Field::Src => self.src as u16,
            Field::Sh => self.sh as u16,
            Field::N => self.n as u16,
            Field::Alu => self.alu as u16,
            Field::Ccx => self.ccx as u16,
            Field::F => self.f as u16,
            Field::Dst => self.dst as u16,
            Field::Wm => self.wm as u16,
            Field::Jfi => self.jfi as u16,
            Field::Cc => self.cc as u16,
            Field::Cha => self.cha as u16,
            Field::Const => self.const_,
        }
    }

    /// Запись поля по идентификатору. Значение усекается до размера поля
    /// (байтовые поля — 0..15 в HEX-представлении файла, CONST — 0..FFFF).
    pub fn set(&mut self, field: Field, value: u16) {
        let v = value & field.mask();
        match field {
            Field::A => self.a = v as u8,
            Field::B => self.b = v as u8,
            Field::Ma => self.ma = v as u8,
            Field::Mb => self.mb = v as u8,
            Field::Mem => self.mem = v as u8,
            Field::Src => self.src = v as u8,
            Field::Sh => self.sh = v as u8,
            Field::N => self.n = v as u8,
            Field::Alu => self.alu = v as u8,
            Field::Ccx => self.ccx = v as u8,
            Field::F => self.f = v as u8,
            Field::Dst => self.dst = v as u8,
            Field::Wm => self.wm = v as u8,
            Field::Jfi => self.jfi = v as u8,
            Field::Cc => self.cc = v as u8,
            Field::Cha => self.cha = v as u8,
            Field::Const => self.const_ = v,
        }
    }
}

/// Справочник допустимых значений поля (для контекстного меню редактора).
/// Возвращаются пары (значение, описание).
pub fn field_options(field: Field) -> Vec<(u16, String)> {
    match field {
        Field::A | Field::B => REGISTER_NAMES
            .iter()
            .enumerate()
            .map(|(i, n)| (i as u16, (*n).to_string()))
            .collect(),
        Field::Ma | Field::Mb => vec![
            (0, "Поле A/B".into()),
            (1, "reg1 (RGK 10:8)".into()),
            (2, "reg2 (RGK 5:3)".into()),
            (3, "r/m (RGK 2:0)".into()),
        ],
        Field::Mem => vec![
            (0, "Нет операции".into()),
            (4, "Чтение байта".into()),
            (5, "Чтение слова".into()),
            (6, "Запись байта".into()),
            (7, "Запись слова".into()),
        ],
        Field::Src => vec![
            (0, "0, 0".into()),
            (1, "RGA, RGB".into()),
            (2, "RGA, RGQ".into()),
            (3, "RGA, RGR".into()),
            (4, "RGA*2, RGB".into()),
            (5, "CONST, RGB".into()),
            (6, "CONST, RGR".into()),
            (7, "CONST, RGQ".into()),
        ],
        Field::Sh => vec![
            (0, "Без сдвига".into()),
            (1, "АС АЛУ вправо".into()),
            (2, "ЛС АЛУ вправо".into()),
            (3, "АС АЛУ, RGQ вправо".into()),
            (4, "ЛС АЛУ, RGQ вправо".into()),
            (5, "ЛС RGQ вправо".into()),
            (6, "RGQ <= ALU".into()),
            (8, "ЛС АЛУ влево".into()),
            (0xA, "ЛС АЛУ, RGQ влево".into()),
            (0xE, "Расширение знака байта".into()),
        ],
        Field::Alu => vec![
            (0, "0".into()),
            (1, "S - R - 1 + C0".into()),
            (2, "R - S - 1 + C0".into()),
            (3, "R + S + C0".into()),
            (4, "S + C0".into()),
            (5, "not S + C0".into()),
            (6, "R + C0".into()),
            (7, "not R + C0".into()),
            (8, "S (умножение, не реализ.)".into()),
            (9, "R & S".into()),
            (0xA, "R & not S".into()),
            (0xB, "not (R & S)".into()),
            (0xC, "R | S".into()),
            (0xD, "not (R | S)".into()),
            (0xE, "R ^ S".into()),
            (0xF, "not (R ^ S)".into()),
        ],
        Field::Ccx => vec![(0, "0".into()), (1, "1".into()), (2, "C (RFI.0)".into()), (3, "C (RFI.0)".into())],
        Field::F => vec![(0, "Не сохранять флаги".into()), (1, "RFI -> RFD".into())],
        Field::Dst => vec![
            (0, "Без записи".into()),
            (1, "РЗУ = RGR".into()),
            (2, "РЗУ(ст.) = RGR(мл.)<<8".into()),
            (3, "РЗУ(мл.) = RGR(ст.)>>8".into()),
            (4, "РЗУ = SDA".into()),
        ],
        Field::Wm => vec![
            (0, "Без записи".into()),
            (1, "RGW = SDA".into()),
            (2, "ARAM = SDA".into()),
            (3, "ARAM = RGB".into()),
        ],
        Field::Jfi => vec![
            (0, "RFI, без инверсии".into()),
            (1, "RFI, инверсия".into()),
            (2, "RFD, без инверсии".into()),
            (3, "RFD, инверсия".into()),
            (4, "Безусловный переход".into()),
            (5, "Останов процессора".into()),
        ],
        Field::Cc => vec![
            (0, "JP: P = 1".into()),
            (1, "JZ: Z = 1".into()),
            (2, "JS: N = 1".into()),
            (3, "JO: V = 1".into()),
            (4, "JC: C = 1".into()),
            (5, "JL: N ^ V = 1".into()),
            (6, "JLE: Z | (N ^ V) = 1".into()),
            (7, "JBE: C | Z = 1".into()),
        ],
        Field::Cha => vec![
            (0, "JZ: CMK <- 0".into()),
            (1, "CJS: вызов подпрограммы".into()),
            (2, "JMAP: дешифрация RGK".into()),
            (3, "CJP: переход по условию".into()),
            (4, "RPCT: цикл по RACT".into()),
            (5, "CRTN: возврат из подпрограммы".into()),
            (6, "LDCT: RACT = CONST".into()),
            (7, "CONT: следующая МК".into()),
        ],
        Field::N => vec![],
        Field::Const => vec![],
    }
}

/// Имена РОНов для полей A/B (индексы 0..15).
pub const REGISTER_NAMES: [&str; 16] = [
    "AX", "CX", "DX", "BX", "SP", "BP", "SI", "DI", "CS", "SS", "DS", "ES", "IP", "PSW", "RGK", "RW",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Валидация ввода согласована со справочником: допустимы ровно те
    /// коды (после усечения), которые есть в field_options; поля без
    /// справочника (N, CONST) принимают любые значения.
    #[test]
    fn input_valid_matches_field_options() {
        for field in Field::ALL {
            for value in 0..32u16 {
                let truncated = value & field.mask();
                let in_options = field_options(field)
                    .iter()
                    .any(|(allowed, _)| *allowed == truncated);
                let expected =
                    matches!(field, Field::N | Field::Const) || in_options;
                assert_eq!(
                    field.input_valid(value),
                    expected,
                    "{field:?}: значение {value:X}"
                );
            }
        }
    }

    /// Отвергаемые коды из задачи: SH — 7, 9, B, C, D, F; MEM — 1, 2, 3.
    #[test]
    fn rejects_known_invalid_codes() {
        for v in [0x7, 0x9, 0xB, 0xC, 0xD, 0xF] {
            assert!(!Field::Sh.input_valid(v), "SH {v:X} должен отвергаться");
        }
        for v in [0x1, 0x2, 0x3] {
            assert!(!Field::Mem.input_valid(v), "MEM {v:X} должен отвергаться");
        }
        for v in [0x5, 0x6, 0x7] {
            assert!(!Field::Dst.input_valid(v), "DST {v:X} должен отвергаться");
        }
        for v in [0x6, 0x7] {
            assert!(!Field::Jfi.input_valid(v), "JFI {v:X} должен отвергаться");
        }
    }
}
