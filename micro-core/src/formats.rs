//! Чтение и запись файлов данных симулятора в форматах оригинала
//! (.MEM, .XMEM, .RAM, .RG, .TTA). Форматы побайтово совместимы с
//! Delphi-версией («Micro classic») и C#-версией (см. docs/ANALYSIS.md §4).

use crate::microcommand::MicroCommand;
use crate::mpm::MPM_SIZE;
use crate::ram::RAM_SIZE;
use crate::register::RegId;
use crate::translation::{AddressTranslationTable, TABLE_ROWS, TableRow};

const SEP: u8 = 0x01; // \u0001 (разделитель полей .MEM/.XMEM)
const STX: u8 = 0x02; // \u0002 (начало записи .RAM/.TTA)
const EOT: u8 = 0x04; // \u0004 (перед CONST в .MEM)
const DLE: u8 = 0x10; // \u0010 (перед опкодом в .TTA)

/// Ошибка разбора файла.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatError {
    pub message: String,
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FormatError {}

fn err<T>(message: impl Into<String>) -> Result<T, FormatError> {
    Err(FormatError {
        message: message.into(),
    })
}

/// Разбор HEX-числа (без знака), регистр букв не важен.
fn parse_hex_u32(s: &str) -> Option<u32> {
    if s.is_empty() || s.len() > 8 {
        return None;
    }
    u32::from_str_radix(s, 16).ok()
}

// ---------------------------------------------------------------------------
// .MEM — классический формат Delphi (17 полей на МК, без комментариев)
// ---------------------------------------------------------------------------

/// Чтение .MEM: поля разделены `\x01`, перед CONST — `\x04`.
/// Разбор устойчив к пробелам (как в оригинале, SEP/EOT заменяются пробелами).
pub fn parse_mem(data: &str) -> Result<Vec<MicroCommand>, FormatError> {
    let cleaned = data.replace(['\u{1}', '\u{4}'], " ");
    let fields: Vec<&str> = cleaned.split_whitespace().collect();
    if fields.is_empty() {
        return err("Файл микропрограммной памяти пуст или повреждён");
    }
    if !fields.len().is_multiple_of(17) {
        return err(format!(
            "Файл микропрограммной памяти повреждён: число полей ({}) не кратно 17",
            fields.len()
        ));
    }
    let count = fields.len() / 17;
    let mut commands = Vec::with_capacity(count.min(MPM_SIZE));
    for i in 0..count {
        let f = |j: usize| -> Result<u8, FormatError> {
            let v = parse_hex_u32(fields[i * 17 + j])
                .ok_or_else(|| FormatError {
                    message: format!(
                        "Файл микропрограммной памяти повреждён: поле {} МК {} — «{}» не является HEX",
                        j,
                        i,
                        fields[i * 17 + j]
                    ),
                })?;
            if v > 0xFF {
                return err(format!(
                    "Файл микропрограммной памяти повреждён: поле {} МК {} вне диапазона байта",
                    j, i
                ));
            }
            Ok(v as u8)
        };
        let mut mk = MicroCommand::new(i as u8);
        mk.a = f(0)?;
        mk.b = f(1)?;
        mk.ma = f(2)?;
        mk.mb = f(3)?;
        mk.mem = f(4)?;
        mk.src = f(5)?;
        mk.sh = f(6)?;
        mk.n = f(7)?;
        mk.alu = f(8)?;
        mk.ccx = f(9)?;
        mk.f = f(10)?;
        mk.dst = f(11)?;
        mk.wm = f(12)?;
        mk.jfi = f(13)?;
        mk.cc = f(14)?;
        mk.cha = f(15)?;
        mk.const_ = parse_hex_u32(fields[i * 17 + 16]).unwrap_or(0) as u16;
        commands.push(mk);
    }
    Ok(commands)
}

/// Запись .MEM (как в оригинале: `\x01` перед каждым полем, `\x04` перед CONST).
pub fn write_mem(commands: &[MicroCommand]) -> String {
    let mut out = String::with_capacity(commands.len() * 40);
    for mk in commands {
        for (v, width) in [
            (mk.a as u32, 1),
            (mk.b as u32, 1),
            (mk.ma as u32, 1),
            (mk.mb as u32, 1),
            (mk.mem as u32, 1),
            (mk.src as u32, 1),
            (mk.sh as u32, 1),
            (mk.n as u32, 1),
            (mk.alu as u32, 1),
            (mk.ccx as u32, 1),
            (mk.f as u32, 1),
            (mk.dst as u32, 1),
            (mk.wm as u32, 1),
            (mk.jfi as u32, 1),
            (mk.cc as u32, 1),
            (mk.cha as u32, 1),
        ] {
            out.push(SEP as char);
            out.push_str(&format!("{:0width$X}", v, width = width));
        }
        out.push(EOT as char);
        out.push_str(&format!("{:04X}", mk.const_));
    }
    out
}

// ---------------------------------------------------------------------------
// .XMEM — расширенный формат C#-версии (18-е поле — комментарий)
// ---------------------------------------------------------------------------

/// Чтение .XMEM: поля через `\x01`, строки через `\n`, каждые 18 полей — МК.
pub fn parse_xmem(data: &str) -> Result<Vec<MicroCommand>, FormatError> {
    let fields: Vec<&str> = data.split(['\u{1}', '\n']).collect();
    if fields.len() < 18 {
        return err("Файл .XMEM повреждён: недостаточно полей");
    }
    let count = fields.len() / 18;
    let mut commands = Vec::with_capacity(count.min(MPM_SIZE));
    for i in 0..count {
        let f = |j: usize| -> Result<u8, FormatError> {
            let s = fields[i * 18 + j];
            let v = parse_hex_u32(s).ok_or_else(|| FormatError {
                message: format!(
                    "Файл .XMEM повреждён: поле {} МК {} — «{}» не является HEX",
                    j, i, s
                ),
            })?;
            if v > 0xFF {
                return err(format!(
                    "Файл .XMEM повреждён: поле {} МК {} вне диапазона байта",
                    j, i
                ));
            }
            Ok(v as u8)
        };
        let mut mk = MicroCommand::new(i as u8);
        mk.a = f(0)?;
        mk.b = f(1)?;
        mk.ma = f(2)?;
        mk.mb = f(3)?;
        mk.mem = f(4)?;
        mk.src = f(5)?;
        mk.sh = f(6)?;
        mk.n = f(7)?;
        mk.alu = f(8)?;
        mk.ccx = f(9)?;
        mk.f = f(10)?;
        mk.dst = f(11)?;
        mk.wm = f(12)?;
        mk.jfi = f(13)?;
        mk.cc = f(14)?;
        mk.cha = f(15)?;
        mk.const_ = parse_hex_u32(fields[i * 18 + 16]).unwrap_or(0) as u16;
        mk.comment = fields[i * 18 + 17].to_string();
        commands.push(mk);
    }
    Ok(commands)
}

/// Запись .XMEM (18 полей на строку, 18-е — комментарий).
pub fn write_xmem(commands: &[MicroCommand]) -> String {
    let mut out = String::with_capacity(commands.len() * 64);
    for mk in commands {
        for (v, width) in [
            (mk.a as u32, 1),
            (mk.b as u32, 1),
            (mk.ma as u32, 1),
            (mk.mb as u32, 1),
            (mk.mem as u32, 1),
            (mk.src as u32, 1),
            (mk.sh as u32, 1),
            (mk.n as u32, 1),
            (mk.alu as u32, 1),
            (mk.ccx as u32, 1),
            (mk.f as u32, 1),
            (mk.dst as u32, 1),
            (mk.wm as u32, 1),
            (mk.jfi as u32, 1),
            (mk.cc as u32, 1),
            (mk.cha as u32, 1),
        ] {
            out.push_str(&format!("{:0width$X}", v, width = width));
            out.push(SEP as char);
        }
        out.push_str(&format!("{:04X}", mk.const_));
        out.push(SEP as char);
        out.push_str(&mk.comment);
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// .RAM — ОЗУ (`\x02` + 2 HEX-цифры на байт)
// ---------------------------------------------------------------------------

pub fn parse_ram(data: &[u8]) -> Result<Vec<u8>, FormatError> {
    let mut bytes = Vec::with_capacity(RAM_SIZE);
    let mut expect_value = false;
    let mut hi: Option<u8> = None;
    for &b in data {
        if b == STX {
            expect_value = true;
            hi = None;
            continue;
        }
        if !expect_value {
            continue; // пропускаем мусор между записями (пробелы и т.п.)
        }
        let digit = match (b as char).to_digit(16) {
            Some(d) => d as u8,
            None => {
                return err(format!(
                    "Файл оперативной памяти повреждён: неожиданный символ «{}»",
                    b as char
                ))
            }
        };
        match hi {
            None => hi = Some(digit),
            Some(h) => {
                bytes.push((h << 4) | digit);
                hi = None;
                expect_value = false;
            }
        }
    }
    Ok(bytes)
}

pub fn write_ram(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 3);
    for &b in bytes {
        out.push(STX as char);
        out.push_str(&format!("{:02X}", b));
    }
    out
}

// ---------------------------------------------------------------------------
// .RG — значения регистров (154 символа, совместимость с Delphi)
// ---------------------------------------------------------------------------

const RG_LEN: usize = 154;

/// Чтение .RG. Возвращает пары (регистр, значение). RFI/RFD из файла
/// не читаются (как в оригинале — см. ANALYSIS.md §4).
pub fn parse_rg(data: &str) -> Result<Vec<(RegId, u16)>, FormatError> {
    let bytes = data.as_bytes();
    if bytes.len() != RG_LEN {
        return err(format!(
            "Файл значений регистров повреждён: ожидается {} символов, получено {}",
            RG_LEN,
            bytes.len()
        ));
    }
    let sub = |offset: usize| -> Result<u16, FormatError> {
        let s = std::str::from_utf8(&bytes[offset..offset + 4]).map_err(|_| FormatError {
            message: "Файл значений регистров повреждён".into(),
        })?;
        parse_hex_u32(s)
            .map(|v| v as u16)
            .ok_or_else(|| FormatError {
                message: format!(
                    "Файл значений регистров повреждён: «{}» по смещению {} не является HEX",
                    s, offset
                ),
            })
    };
    let mut result = Vec::with_capacity(28);
    // РОНы: AX@1, BX@6, CX@11, DX@16, затем SP..RW по i*5+1
    result.push((RegId::Ax, sub(1)?));
    result.push((RegId::Cx, sub(11)?));
    result.push((RegId::Dx, sub(16)?));
    result.push((RegId::Bx, sub(6)?));
    for i in 4..16 {
        let reg = RegId::from_index(i).unwrap();
        result.push((reg, sub(i * 5 + 1)?));
    }
    result.push((RegId::Rgq, sub(87)?));
    result.push((RegId::Cmk, sub(92)?));
    result.push((RegId::Muad, sub(97)?));
    result.push((RegId::Stp, sub(102)?));
    result.push((RegId::Error, sub(107)?));
    result.push((RegId::Ract, sub(112)?));
    result.push((RegId::Aram, sub(120)?));
    result.push((RegId::Rgr, sub(125)?));
    result.push((RegId::Rgw, sub(130)?));
    Ok(result)
}

/// Запись .RG (154 символа; RFI/RFD не сохраняются — как в оригинале).
pub fn write_rg(regs: &crate::register::RegisterMemory) -> String {
    let mut out = String::with_capacity(RG_LEN);
    // EOT + значения: AX, BX, CX, DX, затем индексы 4..15
    let push_reg = |out: &mut String, reg: RegId| {
        out.push(EOT as char);
        out.push_str(&format!("{:04X}", regs.get(reg)));
    };
    push_reg(&mut out, RegId::Ax);
    push_reg(&mut out, RegId::Bx);
    push_reg(&mut out, RegId::Cx);
    push_reg(&mut out, RegId::Dx);
    for i in 4..16 {
        push_reg(&mut out, RegId::from_index(i).unwrap());
    }
    // STX 00 ×2 (места RFI/RFD), затем RGQ, CMK, MUAD, STP, ERROR, RACT
    out.push(STX as char);
    out.push_str("00");
    out.push(STX as char);
    out.push_str("00");
    push_reg(&mut out, RegId::Rgq);
    push_reg(&mut out, RegId::Cmk);
    push_reg(&mut out, RegId::Muad);
    push_reg(&mut out, RegId::Stp);
    push_reg(&mut out, RegId::Error);
    push_reg(&mut out, RegId::Ract);
    out.push('\0');
    out.push(SEP as char);
    out.push(SEP as char);
    push_reg(&mut out, RegId::Aram);
    push_reg(&mut out, RegId::Rgr);
    push_reg(&mut out, RegId::Rgw);
    // 20 нулевых символов
    out.push_str("\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
    out
}

// ---------------------------------------------------------------------------
// .TTA — таблица преобразования адресов (480 символов)
// ---------------------------------------------------------------------------

const TTA_LEN: usize = 480;
const TTA_ROW: usize = 24;

pub fn parse_tta(data: &str) -> Result<AddressTranslationTable, FormatError> {
    let bytes = data.as_bytes();
    if bytes.len() != TTA_LEN {
        return err(format!(
            "Файл таблицы преобразования адресов повреждён: ожидается {} символов, получено {}",
            TTA_LEN,
            bytes.len()
        ));
    }
    let mut table = AddressTranslationTable::new();
    for i in 0..TABLE_ROWS {
        let addr_s = std::str::from_utf8(&bytes[i * TTA_ROW + 1..i * TTA_ROW + 3]).map_err(|_| {
            FormatError {
                message: "Файл таблицы преобразования адресов повреждён".into(),
            }
        })?;
        let opcode =
            std::str::from_utf8(&bytes[i * TTA_ROW + 4..i * TTA_ROW + 20]).map_err(|_| {
                FormatError {
                    message: "Файл таблицы преобразования адресов повреждён".into(),
                }
            })?;
        let address = parse_hex_u32(addr_s).ok_or_else(|| FormatError {
            message: format!("Таблица преобразования адресов повреждена: строка {}", i + 1),
        })? as u8;
        table.rows[i] = TableRow {
            address,
            opcode: opcode.to_string(),
        };
    }
    Ok(table)
}

pub fn write_tta(table: &AddressTranslationTable) -> String {
    let mut out = String::with_capacity(TTA_LEN);
    for row in &table.rows {
        out.push(STX as char);
        out.push_str(&format!("{:02X}", row.address));
        out.push(DLE as char);
        out.push_str(&row.opcode);
        out.push_str("MaxA");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::microcommand::Field;

    #[test]
    fn mem_roundtrip() {
        let mut commands: Vec<MicroCommand> = (0..MPM_SIZE as u8).map(MicroCommand::new).collect();
        commands[0].a = 0xF;
        commands[0].const_ = 0xABCD;
        commands[1].src = 7;
        commands[63].cha = 0;
        let s = write_mem(&commands);
        let parsed = parse_mem(&s).unwrap();
        assert_eq!(parsed.len(), MPM_SIZE);
        assert_eq!(parsed[0].a, 0xF);
        assert_eq!(parsed[0].const_, 0xABCD);
        assert_eq!(parsed[1].src, 7);
        assert_eq!(parsed[63].cha, 0);
    }

    #[test]
    fn mem_rejects_bad_length() {
        assert!(parse_mem("0 1 2").is_err());
    }

    #[test]
    fn xmem_roundtrip_with_comments() {
        let mut commands: Vec<MicroCommand> = (0..4).map(MicroCommand::new).collect();
        commands[2].comment = "Комментарий с кириллицей".to_string();
        commands[2].alu = 0xE;
        let s = write_xmem(&commands);
        let parsed = parse_xmem(&s).unwrap();
        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed[2].comment, "Комментарий с кириллицей");
        assert_eq!(parsed[2].alu, 0xE);
    }

    #[test]
    fn ram_roundtrip() {
        let mut bytes = vec![0u8; RAM_SIZE];
        bytes[0] = 0xDE;
        bytes[1023] = 0xAD;
        let s = write_ram(&bytes);
        assert_eq!(s.len(), RAM_SIZE * 3);
        let parsed = parse_ram(s.as_bytes()).unwrap();
        assert_eq!(parsed, bytes);
    }

    #[test]
    fn rg_roundtrip() {
        let mut regs = crate::register::RegisterMemory::new();
        regs.set_silent(RegId::Ax, 0x0008);
        regs.set_silent(RegId::Bx, 0x0002);
        regs.set_silent(RegId::Cx, 0x0020);
        regs.set_silent(RegId::Rgq, 0x1A10);
        regs.set_silent(RegId::Cmk, 0x0008);
        let s = write_rg(&regs);
        assert_eq!(s.len(), RG_LEN, "длина файла .RG должна быть ровно 154");
        let parsed = parse_rg(&s).unwrap();
        let map: std::collections::HashMap<RegId, u16> = parsed.into_iter().collect();
        assert_eq!(map[&RegId::Ax], 0x0008);
        assert_eq!(map[&RegId::Bx], 0x0002);
        assert_eq!(map[&RegId::Cx], 0x0020);
        assert_eq!(map[&RegId::Rgq], 0x1A10);
        assert_eq!(map[&RegId::Cmk], 0x0008);
        assert!(!map.contains_key(&RegId::Rfi));
    }

    #[test]
    fn tta_roundtrip() {
        let mut table = AddressTranslationTable::new();
        table.rows[0] = TableRow::new(0x1F, "0000XX1111000000");
        table.rows[19] = TableRow::new(0x3F, "1111111111111111");
        let s = write_tta(&table);
        assert_eq!(s.len(), TTA_LEN);
        let parsed = parse_tta(&s).unwrap();
        assert_eq!(parsed.rows[0], table.rows[0]);
        assert_eq!(parsed.rows[19], table.rows[19]);
    }

    /// Тест на реальном файле лабораторной работы №4 (example.MEM).
    #[test]
    fn parse_real_lab_mem() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/example.MEM"
        );
        let Ok(data) = std::fs::read_to_string(path) else {
            eprintln!("пропущен: {} отсутствует", path);
            return;
        };
        let commands = parse_mem(&data).expect("example.MEM должен разбираться");
        assert_eq!(commands.len(), MPM_SIZE);
    }

    #[test]
    fn parse_real_lab_ram() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/example.RAM");
        let Ok(data) = std::fs::read(path) else {
            eprintln!("пропущен: {} отсутствует", path);
            return;
        };
        let bytes = parse_ram(&data).expect("example.RAM должен разбираться");
        assert_eq!(bytes.len(), RAM_SIZE);
    }

    #[test]
    fn parse_real_lab_rg() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/example.RG");
        let Ok(data) = std::fs::read_to_string(path) else {
            eprintln!("пропущен: {} отсутствует", path);
            return;
        };
        let regs = parse_rg(&data).expect("example.RG должен разбираться");
        assert!(!regs.is_empty());
    }

    #[test]
    fn parse_real_lab_tta() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/example.TTA");
        let Ok(data) = std::fs::read_to_string(path) else {
            eprintln!("пропущен: {} отсутствует", path);
            return;
        };
        let table = parse_tta(&data).expect("example.TTA должен разбираться");
        assert_eq!(table.rows.len(), TABLE_ROWS);
    }

    /// Поле-агностик проверка: set/get по всем полям.
    #[test]
    fn field_access_roundtrip() {
        let mut mk = MicroCommand::new(3);
        for field in Field::ALL {
            let max: u16 = match field {
                Field::Const => 0xFFFF,
                Field::A | Field::B | Field::N | Field::Alu | Field::Sh => 0xF,
                Field::Ma | Field::Mb | Field::Ccx | Field::Wm => 0x3,
                Field::Mem | Field::Src | Field::Cc | Field::Cha | Field::Jfi | Field::Dst => 0x7,
                Field::F => 0x1,
            };
            mk.set(field, max);
            assert_eq!(mk.get(field), max, "поле {:?}", field);
        }
        assert_eq!(mk.address, 3);
    }
}
