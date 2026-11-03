//! Регистровая память процессора: 31 регистр.
//!
//! Индексы 0..15 («РОНы») доступны полям микрокоманды A/B/reg1/reg2/r/m.
//! Порядок следования и семантика индексов соответствуют оригинальной C#-версии
//! (`RegisterMemory` + порядок словаря в `MicroViewModel`), чтобы поведение
//! полей MA/MB и форматов файлов совпадало бит-в-бит.

/// Идентификатор регистра. Дискриминанты = индексы в порядке следования UI/файлов.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RegId {
    Ax = 0,
    Cx = 1,
    Dx = 2,
    Bx = 3,
    Sp = 4,
    Bp = 5,
    Si = 6,
    Di = 7,
    Cs = 8,
    Ss = 9,
    Ds = 10,
    Es = 11,
    Ip = 12,
    Psw = 13,
    Rgk = 14,
    Rw = 15,
    Rfi = 16,
    Rfd = 17,
    Rgq = 18,
    Aram = 19,
    Rgr = 20,
    Rgw = 21,
    Ract = 22,
    Cmk = 23,
    Muad = 24,
    Stp = 25,
    Error = 26,
    Rga = 27,
    Rgb = 28,
    Alu = 29,
    Sda = 30,
}

/// Все регистры в каноническом порядке (индекс = дискриминант).
pub const ALL: [RegId; 31] = [
    RegId::Ax,
    RegId::Cx,
    RegId::Dx,
    RegId::Bx,
    RegId::Sp,
    RegId::Bp,
    RegId::Si,
    RegId::Di,
    RegId::Cs,
    RegId::Ss,
    RegId::Ds,
    RegId::Es,
    RegId::Ip,
    RegId::Psw,
    RegId::Rgk,
    RegId::Rw,
    RegId::Rfi,
    RegId::Rfd,
    RegId::Rgq,
    RegId::Aram,
    RegId::Rgr,
    RegId::Rgw,
    RegId::Ract,
    RegId::Cmk,
    RegId::Muad,
    RegId::Stp,
    RegId::Error,
    RegId::Rga,
    RegId::Rgb,
    RegId::Alu,
    RegId::Sda,
];

impl RegId {
    pub const fn name(self) -> &'static str {
        match self {
            RegId::Ax => "AX",
            RegId::Cx => "CX",
            RegId::Dx => "DX",
            RegId::Bx => "BX",
            RegId::Sp => "SP",
            RegId::Bp => "BP",
            RegId::Si => "SI",
            RegId::Di => "DI",
            RegId::Cs => "CS",
            RegId::Ss => "SS",
            RegId::Ds => "DS",
            RegId::Es => "ES",
            RegId::Ip => "IP",
            RegId::Psw => "PSW",
            RegId::Rgk => "RGK",
            RegId::Rw => "RW",
            RegId::Rfi => "RFI",
            RegId::Rfd => "RFD",
            RegId::Rgq => "RGQ",
            RegId::Aram => "ARAM",
            RegId::Rgr => "RGR",
            RegId::Rgw => "RGW",
            RegId::Ract => "RACT",
            RegId::Cmk => "CMK",
            RegId::Muad => "MUAD",
            RegId::Stp => "STP",
            RegId::Error => "ERROR",
            RegId::Rga => "RGA",
            RegId::Rgb => "RGB",
            RegId::Alu => "ALU",
            RegId::Sda => "SDA",
        }
    }

    /// По индексу 0..30 (канонический порядок). `None` вне диапазона.
    pub const fn from_index(index: usize) -> Option<RegId> {
        if index < ALL.len() {
            Some(ALL[index])
        } else {
            None
        }
    }

    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Регистровая память: плоский массив на 31 значение.
#[derive(Debug, Clone)]
pub struct RegisterMemory {
    values: [u16; 31],
    /// Битовая маска регистров, изменённых после последнего `take_changes()`
    /// (используется UI для подсветки).
    changed: [bool; 31],
}

impl Default for RegisterMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterMemory {
    pub const fn new() -> Self {
        Self {
            values: [0u16; 31],
            changed: [false; 31],
        }
    }

    #[inline]
    pub fn get(&self, reg: RegId) -> u16 {
        self.values[reg.index()]
    }

    #[inline]
    pub fn set(&mut self, reg: RegId, value: u16) {
        let i = reg.index();
        if self.values[i] != value {
            self.values[i] = value;
            self.changed[i] = true;
        }
    }

    /// Запись без пометки «изменено» (используется загрузкой из файла/сбросом).
    #[inline]
    pub fn set_silent(&mut self, reg: RegId, value: u16) {
        self.values[reg.index()] = value;
    }

    /// Индексный доступ 0..15, как в полях микрокоманды (reg1/reg2/r/m).
    /// Вне 0..15 возвращается 0 (в оригинале индекс ограничен полями МК ≤ 7).
    #[inline]
    pub fn get_by_index(&self, index: u16) -> u16 {
        match RegId::from_index(index as usize) {
            Some(reg) if (reg as u8) < 16 => self.values[reg.index()],
            _ => 0,
        }
    }

    #[inline]
    pub fn set_by_index(&mut self, index: u16, value: u16) {
        if let Some(reg) = RegId::from_index(index as usize) {
            if (reg as u8) < 16 {
                self.set(reg, value);
            }
        }
    }

    /// Изменился ли регистр с момента последнего вызова `take_changes`
    /// (для подсветки в UI).
    #[inline]
    pub fn is_changed(&self, reg: RegId) -> bool {
        self.changed[reg.index()]
    }

    /// Снять все отметки об изменении (вызывается при начале новой МК).
    pub fn take_changes(&mut self) {
        self.changed = [false; 31];
    }

    /// Сброс всех регистров в 0.
    pub fn reset_all(&mut self) {
        self.values = [0u16; 31];
        self.changed = [false; 31];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_order_matches_canonical() {
        assert_eq!(RegId::from_index(0).unwrap(), RegId::Ax);
        assert_eq!(RegId::from_index(3).unwrap(), RegId::Bx);
        assert_eq!(RegId::from_index(14).unwrap(), RegId::Rgk);
        assert_eq!(RegId::from_index(16).unwrap(), RegId::Rfi);
        assert_eq!(RegId::from_index(23).unwrap(), RegId::Cmk);
        assert_eq!(RegId::from_index(30).unwrap(), RegId::Sda);
        assert!(RegId::from_index(31).is_none());
    }

    #[test]
    fn change_tracking() {
        let mut regs = RegisterMemory::new();
        regs.set(RegId::Ax, 5);
        assert!(regs.is_changed(RegId::Ax));
        assert!(!regs.is_changed(RegId::Bx));
        regs.take_changes();
        assert!(!regs.is_changed(RegId::Ax));
        regs.set(RegId::Ax, 5); // то же значение — не изменение
        assert!(!regs.is_changed(RegId::Ax));
    }
}
