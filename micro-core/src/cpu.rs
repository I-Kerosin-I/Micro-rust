//! Состояние процессора и исполнение микрокоманд.
//!
//! Семантика исполнения побитово повторяет `CpuState._executeMicroCommand`
//! оригинальной C#-версии, включая 32-битную арифметику АЛУ, формирование
//! флагов и работу управляющего устройства.

use crate::microcommand::MicroCommand;
use crate::mpm::{MicroProgramMemory, MPM_SIZE};
use crate::ram::RamMemory;
use crate::register::{RegId, RegisterMemory};
use crate::translation::AddressTranslationTable;

/// Режим исполнения ЦП (аналог `ExecutionState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState {
    /// Готов к запуску (после рестарта).
    ReadyToStart,
    /// Непрерывное исполнение.
    Running,
    /// Остановлен (JFI=5, CMK за границей МПП или ошибка).
    Stopped,
    /// Пауза (между шагами).
    Paused,
}

/// События, о которых UI должен сообщить пользователю
/// (в оригинале — MessageBox.Show посреди исполнения).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuEvent {
    /// Чтение байта/слова за пределами ОЗУ.
    MemoryReadOutOfBounds { address: u16, word: bool },
    /// Запись байта/слова за пределами ОЗУ.
    MemoryWriteOutOfBounds { address: u16, word: bool },
    /// Дешифрация не удалась: кода операции нет в таблице.
    DecodeFailed,
    /// CRTN при пустом стеке адресов возврата.
    StackEmpty,
    /// Недопустимое поле SH в микрокоманде (выполнено как «без сдвига»).
    InvalidShField,
    /// Превышен предел шагов при исполнении одной команды (защита от зацикливания).
    InstructionLimit,
}

/// Результат одного шага.
#[derive(Debug, Clone)]
pub struct StepReport {
    /// Исполненная микрокоманда (копия для строки «Выполнена»).
    /// `None` — шаг не выполнялся (ЦП остановлен по границе МПП).
    pub executed: Option<MicroCommand>,
    pub events: Vec<CpuEvent>,
}

/// Снимок состояния для трассы: 27 величин в фиксированном порядке.
pub type TraceRow = [u16; 27];

/// Ядро процессора.
#[derive(Debug, Clone)]
pub struct CpuState {
    pub mpm: MicroProgramMemory,
    pub registers: RegisterMemory,
    pub ram: RamMemory,
    pub table: AddressTranslationTable,
    /// Стек адресов возврата.
    pub stack: Vec<u16>,

    pub state: ExecutionState,
    /// Трасса: снимок после рестарта и после каждой микрокоманды.
    pub trace: Vec<TraceRow>,

    /// Признаки «изменено» для предупреждения при выходе.
    pub mpm_modified: bool,
    pub ram_modified: bool,
    pub tta_modified: bool,
}

impl Default for CpuState {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuState {
    pub fn new() -> Self {
        let mut state = Self {
            mpm: MicroProgramMemory::new(),
            registers: RegisterMemory::new(),
            ram: RamMemory::new(),
            table: AddressTranslationTable::new(),
            stack: Vec::new(),
            state: ExecutionState::ReadyToStart,
            trace: Vec::new(),
            mpm_modified: false,
            ram_modified: false,
            tta_modified: false,
        };
        state.snapshot_trace();
        state
    }

    fn snapshot_trace(&mut self) {
        let r = &self.registers;
        self.trace.push([
            r.get(RegId::Cmk),
            r.get(RegId::Ax),
            r.get(RegId::Cx),
            r.get(RegId::Dx),
            r.get(RegId::Bx),
            r.get(RegId::Sp),
            r.get(RegId::Bp),
            r.get(RegId::Si),
            r.get(RegId::Di),
            r.get(RegId::Cs),
            r.get(RegId::Ss),
            r.get(RegId::Ds),
            r.get(RegId::Es),
            r.get(RegId::Ip),
            r.get(RegId::Psw),
            r.get(RegId::Rgk),
            r.get(RegId::Rw),
            r.get(RegId::Rga),
            r.get(RegId::Rgb),
            r.get(RegId::Alu),
            r.get(RegId::Sda),
            r.get(RegId::Rgq),
            r.get(RegId::Rfi),
            r.get(RegId::Aram),
            r.get(RegId::Rgr),
            r.get(RegId::Rgw),
            r.get(RegId::Ract),
        ]);
    }

    /// Рестарт ЦП: сброс служебных регистров, стека и трассы.
    /// РОНы (AX..RW) и RGQ/ARAM/RGR/RGW/RACT — как в оригинале — не сбрасываются.
    pub fn restart(&mut self) {
        self.state = ExecutionState::ReadyToStart;
        let r = &mut self.registers;
        r.set(RegId::Cmk, 0);
        r.set(RegId::Rfi, 0);
        r.set(RegId::Rfd, 0);
        r.set(RegId::Rga, 0);
        r.set(RegId::Rgb, 0);
        r.set(RegId::Alu, 0);
        r.set(RegId::Sda, 0);
        self.stack.clear();
        self.trace.clear();
        self.snapshot_trace();
    }

    /// Один шаг: исполнить микрокоманду по адресу CMK.
    pub fn step(&mut self) -> StepReport {
        let cmk = self.registers.get(RegId::Cmk);
        if cmk as usize >= MPM_SIZE {
            self.state = ExecutionState::Stopped;
            return StepReport {
                executed: None,
                events: Vec::new(),
            };
        }

        // Начало новой МК: подсветка изменений сбрасывается (аналог
        // MicroCommandStarted в оригинале).
        self.registers.take_changes();
        self.ram.take_changes();

        let mk = self.mpm.get(cmk as usize).clone();
        let events = self.execute(&mk);

        self.snapshot_trace();

        if mk.jfi == 5 || self.registers.get(RegId::Cmk) as usize >= MPM_SIZE {
            self.state = ExecutionState::Stopped;
        }

        StepReport {
            executed: Some(mk),
            events,
        }
    }

    /// Исполнение текущей команды целиком: до МК с CHA=0 включительно.
    /// Защита от зацикливания: не более [`INSTRUCTION_STEP_LIMIT`] шагов.
    pub fn step_instruction(&mut self) -> StepReport {
        const INSTRUCTION_STEP_LIMIT: usize = 65_536;
        // Исполняем, пока следующая МК не выборка команды (CHA != 0).
        let mut combined_events = Vec::new();
        let mut steps = 0usize;
        loop {
            if self.state == ExecutionState::Stopped {
                return StepReport {
                    executed: None,
                    events: combined_events,
                };
            }
            let cmk = self.registers.get(RegId::Cmk) as usize;
            if cmk >= MPM_SIZE || self.mpm.get(cmk).cha == 0 {
                break;
            }
            if steps >= INSTRUCTION_STEP_LIMIT {
                combined_events.push(CpuEvent::InstructionLimit);
                self.state = ExecutionState::Stopped;
                return StepReport {
                    executed: None,
                    events: combined_events,
                };
            }
            let report = self.step();
            combined_events.extend(report.events);
            steps += 1;
            if self.state == ExecutionState::Stopped {
                return StepReport {
                    executed: report.executed,
                    events: combined_events,
                };
            }
        }
        // Выполняем саму МК выборки (CHA = 0) и ставим паузу.
        let report = self.step();
        combined_events.extend(report.events);
        if self.state != ExecutionState::Stopped {
            self.state = ExecutionState::Paused;
        }
        StepReport {
            executed: report.executed,
            events: combined_events,
        }
    }

    /// Исполнение одной микрокоманды (вся логика `_executeMicroCommand`).
    fn execute(&mut self, mk: &MicroCommand) -> Vec<CpuEvent> {
        let mut events = Vec::new();
        let rgk = self.registers.get(RegId::Rgk);

        // --- A, MA: источник операнда A ---
        let a_val = match mk.ma {
            0 => self.registers.get_by_index(mk.a as u16),
            1 => self.registers.get_by_index((rgk & 0b0000_0111_0000_0000) >> 8),
            2 => self.registers.get_by_index((rgk & 0b0000_0000_0011_1000) >> 3),
            3 => self.registers.get_by_index(rgk & 0b0000_0000_0000_0111),
            _ => self.registers.get_by_index(mk.a as u16),
        };
        self.registers.set(RegId::Rga, a_val);

        // --- B, MB: источник/приёмник операнда B ---
        let dst: u16 = match mk.mb {
            1 => (rgk & 0b0000_0111_0000_0000) >> 8,
            2 => (rgk & 0b0000_0000_0011_1000) >> 3,
            3 => rgk & 0b0000_0000_0000_0111,
            _ => mk.b as u16,
        };
        let b_val = self.registers.get_by_index(dst);
        self.registers.set(RegId::Rgb, b_val);

        // --- SRC: коммутатор операндов АЛУ ---
        let rga = self.registers.get(RegId::Rga);
        let rgb = self.registers.get(RegId::Rgb);
        let rgq = self.registers.get(RegId::Rgq);
        let rgr = self.registers.get(RegId::Rgr);
        let (r, s): (u16, u16) = match mk.src {
            0 => (0, 0),
            1 => (rga, rgb),
            2 => (rga, rgq),
            3 => (rga, rgr),
            4 => ((rga << 1), rgb),
            5 => (mk.const_, rgb),
            6 => (mk.const_, rgr),
            7 => (mk.const_, rgq),
            _ => (0, 0),
        };

        // --- CCX: перенос C0 ---
        let c0: u32 = if (mk.ccx & 0b10) == 2 {
            (self.registers.get(RegId::Rfi) & 1) as u32
        } else {
            mk.ccx as u32
        };

        // --- АЛУ: 32-битная арифметика, как в оригинале ---
        let r32 = r as u32;
        let s32 = s as u32;
        let alu32: u32 = match mk.alu {
            0 => 0,
            // (uint)(s - r - 1 + c0): int-арифметика с переносом знака в uint
            1 => ((s as i32).wrapping_sub(r as i32).wrapping_sub(1).wrapping_add(c0 as i32)) as u32,
            2 => ((r as i32).wrapping_sub(s as i32).wrapping_sub(1).wrapping_add(c0 as i32)) as u32,
            3 => ((r as i32).wrapping_add(s as i32).wrapping_add(c0 as i32)) as u32,
            4 => (s as i32).wrapping_add(c0 as i32) as u32,
            5 => (!(s as i32)).wrapping_add(c0 as i32) as u32,
            6 => (r as i32).wrapping_add(c0 as i32) as u32,
            7 => (!(r as i32)).wrapping_add(c0 as i32) as u32,
            8 => s32,
            9 => r32 & s32,
            10 => r32 & !s32,
            11 => !(r32 & s32),
            12 => r32 | s32,
            13 => !(r32 | s32),
            14 => r32 ^ s32,
            15 => !(r32 ^ s32),
            _ => 0,
        };
        let alu = (alu32 & 0x0000_FFFF) as u16;
        self.registers.set(RegId::Alu, alu);

        // --- Формирование флагов (RFI) ---
        let mut rfi: u16 = (alu & 0x8000) >> 8; // N — бит 7
        rfi |= if alu == 0 { 0x40 } else { 0 }; // Z — бит 6

        if mk.alu < 9 {
            // Арифметические операции: формируются C и V
            rfi |= ((alu32 & 0x1_0000) >> 16) as u16; // C — бит 0
            // V — бит 11: N ^ C (в оригинале — хитрое выражение со сдвигами RFI)
            rfi |= ((rfi << 4) ^ (rfi << 11)) & 0x800;
        } else {
            // Логические операции: C и V сбрасываются
            rfi &= 0b1111_1111_1111_1110;
            rfi &= 0b1111_0111_1111_1111;
        }

        // P — бит 2: чётность 16 бит ALU (XOR всех бит)
        let parity = (0..16).fold(0u16, |acc, i| acc ^ ((alu >> i) & 1));
        rfi |= parity << 2;
        self.registers.set(RegId::Rfi, rfi);

        if mk.f == 1 {
            self.registers.set(RegId::Rfd, rfi);
        }

        // --- Сдвигатель (SH) ---
        let sda: u16 = match mk.sh {
            0 => alu,
            1 => ((alu as i16) >> mk.n) as u16, // арифметический вправо
            2 => alu >> mk.n,                   // логический вправо
            3 => {
                self.registers.set(
                    RegId::Rgq,
                    (((rgq as u32) >> mk.n) | ((alu as u32) << (16 - mk.n))) as u16,
                );
                ((alu as i16) >> mk.n) as u16
            }
            4 => {
                self.registers.set(
                    RegId::Rgq,
                    (((rgq as u32) >> mk.n) | ((alu as u32) << (16 - mk.n))) as u16,
                );
                alu >> mk.n
            }
            5 => {
                self.registers.set(RegId::Rgq, ((rgq as u32) >> mk.n) as u16);
                alu
            }
            6 => {
                self.registers.set(RegId::Rgq, alu);
                alu
            }
            8 => (((alu as u32) << mk.n) & 0xFFFF) as u16,
            10 => {
                // SDA = ALU<<N | RGQ>>(16-N); RGQ <<= N (из старого значения RGQ)
                self.registers
                    .set(RegId::Rgq, (((rgq as u32) << mk.n) & 0xFFFF) as u16);
                ((((alu as u32) << mk.n) & 0xFFFF) | ((rgq as u32) >> (16 - mk.n))) as u16
            }
            14 => {
                if (alu & 0x80) == 0x80 {
                    alu | 0xFF00
                } else {
                    alu & 0x00FF
                }
            }
            _ => {
                events.push(CpuEvent::InvalidShField);
                alu
            }
        };
        self.registers.set(RegId::Sda, sda);

        // --- WM: регистры интерфейса ОЗУ ---
        match mk.wm {
            1 => self.registers.set(RegId::Rgw, sda),
            2 => self.registers.set(RegId::Aram, sda),
            3 => self.registers.set(RegId::Aram, rgb),
            _ => {}
        }

        // --- MEM: доступ к ОЗУ ---
        let aram = self.registers.get(RegId::Aram);
        match mk.mem {
            4 => {
                if (aram as usize) < self.ram.size() {
                    let v = self.ram.get(aram);
                    self.registers.set(RegId::Rgr, v as u16);
                } else {
                    events.push(CpuEvent::MemoryReadOutOfBounds { address: aram, word: false });
                }
            }
            5 => {
                if (aram as usize) + 1 < self.ram.size() {
                    let v = self.ram.get_word(aram).unwrap_or(0);
                    self.registers.set(RegId::Rgr, v);
                } else {
                    events.push(CpuEvent::MemoryReadOutOfBounds { address: aram, word: true });
                }
            }
            6 => {
                if (aram as usize) < self.ram.size() {
                    self.ram.set(aram, (self.registers.get(RegId::Rgw) & 0xFF) as u8);
                } else {
                    events.push(CpuEvent::MemoryWriteOutOfBounds { address: aram, word: false });
                }
            }
            7 => {
                if (aram as usize) + 1 < self.ram.size() {
                    self.ram.set_word(aram, self.registers.get(RegId::Rgw));
                } else {
                    events.push(CpuEvent::MemoryWriteOutOfBounds { address: aram, word: true });
                }
            }
            _ => {}
        }

        // --- DST: запись результата в регистр-приёмник ---
        match mk.dst {
            1 => self.registers.set_by_index(dst, self.registers.get(RegId::Rgr)),
            2 => {
                let cur = self.registers.get_by_index(dst);
                let rgr = self.registers.get(RegId::Rgr);
                self.registers.set_by_index(dst, (cur & 0x00FF) | ((rgr << 8) & 0xFF00));
            }
            3 => {
                let cur = self.registers.get_by_index(dst);
                let rgr = self.registers.get(RegId::Rgr);
                self.registers.set_by_index(dst, (cur & 0xFF00) | (rgr >> 8));
            }
            4 => self.registers.set_by_index(dst, sda),
            _ => {}
        }

        // --- Условие перехода ---
        let mut jmp_cond = true;
        if (mk.jfi & 4) == 0 {
            let flags = if (mk.jfi & 2) == 0 {
                self.registers.get(RegId::Rfi)
            } else {
                self.registers.get(RegId::Rfd)
            };
            jmp_cond = match mk.cc {
                0 => (flags & 0x4) != 0,    // P
                1 => (flags & 0x40) != 0,   // Z
                2 => (flags & 0x80) != 0,   // N
                3 => (flags & 0x800) != 0,  // V
                4 => (flags & 0x1) != 0,    // C
                5 => ((flags & 0x80) ^ ((flags & 0x800) >> 4)) != 0, // N ^ V
                6 => ((flags & 0x40) | ((flags & 0x80) ^ ((flags & 0x800) >> 4))) != 0, // Z | (N ^ V)
                7 => ((flags & 0x1) | (flags & 0x40)) != 0, // C | Z
                _ => jmp_cond,
            };
            if (mk.jfi & 1) != 0 {
                jmp_cond = !jmp_cond;
            }
        }

        // --- CHA: операция управляющего устройства ---
        match mk.cha {
            0 => self.registers.set(RegId::Cmk, 0),
            1 => {
                if jmp_cond {
                    self.stack.push((mk.address as u16).wrapping_add(1));
                    self.registers.set(RegId::Cmk, mk.const_);
                } else {
                    self.inc_cmk();
                }
            }
            2 => {
                // JMAP: дешифрация RGK по таблице
                let rgk = self.registers.get(RegId::Rgk);
                match self.table.decode(rgk) {
                    Some(addr) => self.registers.set(RegId::Cmk, addr as u16),
                    None => {
                        events.push(CpuEvent::DecodeFailed);
                        self.registers.set(RegId::Cmk, 0);
                        // Как в оригинале: трасса и событие «выполнена» пропускаются.
                        return events;
                    }
                }
            }
            3 => {
                if jmp_cond {
                    self.registers.set(RegId::Cmk, mk.const_);
                } else {
                    self.inc_cmk();
                }
            }
            4 => {
                if self.registers.get(RegId::Ract) != 0 {
                    let ract = self.registers.get(RegId::Ract);
                    self.registers.set(RegId::Ract, ract - 1);
                    self.registers.set(RegId::Cmk, mk.const_);
                } else {
                    self.inc_cmk();
                }
            }
            5 => {
                if jmp_cond {
                    match self.stack.pop() {
                        Some(addr) => self.registers.set(RegId::Cmk, addr),
                        None => {
                            events.push(CpuEvent::StackEmpty);
                            // Отличие от оригинала: остановка вместо зависания
                            // (в C# CMK не инкрементируется и цикл повторяется вечно).
                            self.state = ExecutionState::Stopped;
                            return events;
                        }
                    }
                } else {
                    self.inc_cmk();
                }
            }
            6 => {
                self.registers.set(RegId::Ract, mk.const_);
                self.inc_cmk();
            }
            7 => self.inc_cmk(),
            _ => self.inc_cmk(),
        }

        events
    }

    #[inline]
    fn inc_cmk(&mut self) {
        let cmk = self.registers.get(RegId::Cmk);
        self.registers.set(RegId::Cmk, cmk.wrapping_add(1));
    }

    /// Глубина стека для отображения (STP).
    pub fn stp(&self) -> u16 {
        self.stack.len() as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::microcommand::Field;

    fn cpu() -> CpuState {
        CpuState::new()
    }

    /// Утилита: выставить МК по адресу и вернуть состояние.
    fn with_mk(cpu: &mut CpuState, addr: usize, f: impl FnOnce(&mut MicroCommand)) {
        f(cpu.mpm.get_mut(addr));
    }

    #[test]
    fn alu_add_with_carry_in() {
        let mut c = cpu();
        with_mk(&mut c, 0, |mk| {
            mk.src = 0; // r=0, s=0
            mk.alu = 3; // R + S + C0
            mk.ccx = 1; // C0 = 1
            mk.cha = 7; // CONT
        });
        c.registers.set(RegId::Cmk, 0);
        c.step();
        assert_eq!(c.registers.get(RegId::Alu), 1);
        // Флаги: Z=0, N=0, C=0, P=1 (один единичный бит)
        let rfi = c.registers.get(RegId::Rfi);
        assert_eq!(rfi, 0x04);
    }

    #[test]
    fn alu_flags_nzcv() {
        let mut c = cpu();
        // AX + BX = 0x7FFF + 0x0001 = 0x8000: N=1, Z=0, C=0, V=1
        c.registers.set(RegId::Ax, 0x7FFF);
        c.registers.set(RegId::Bx, 0x0001);
        with_mk(&mut c, 0, |mk| {
            mk.a = 0; // RGA = AX
            mk.b = 3; // RGB = BX
            mk.src = 1;
            mk.alu = 3;
            mk.ccx = 0;
            mk.cha = 7;
        });
        c.step();
        assert_eq!(c.registers.get(RegId::Alu), 0x8000);
        let rfi = c.registers.get(RegId::Rfi);
        assert_eq!((rfi >> 7) & 1, 1, "N");
        assert_eq!((rfi >> 6) & 1, 0, "Z");
        assert_eq!((rfi >> 11) & 1, 1, "V");
        assert_eq!(rfi & 1, 0, "C");
        // P: 0x8000 имеет один единичный бит -> нечётная -> P=1
        assert_eq!((rfi >> 2) & 1, 1, "P");
    }

    #[test]
    fn alu_carry_out() {
        let mut c = cpu();
        c.registers.set(RegId::Ax, 0xFFFF);
        c.registers.set(RegId::Bx, 0x0002);
        with_mk(&mut c, 0, |mk| {
            mk.a = 0;
            mk.b = 3;
            mk.src = 1;
            mk.alu = 3;
            mk.ccx = 1; // +1 => 0x10002
            mk.cha = 7;
        });
        c.step();
        assert_eq!(c.registers.get(RegId::Alu), 0x0002);
        let rfi = c.registers.get(RegId::Rfi);
        assert_eq!(rfi & 1, 1, "C");
        assert_eq!((rfi >> 6) & 1, 0, "Z (ALU = 0x0002 != 0)");
    }

    #[test]
    fn logical_ops_clear_carry() {
        let mut c = cpu();
        c.registers.set(RegId::Ax, 0xFF00);
        c.registers.set(RegId::Bx, 0x0FF0);
        with_mk(&mut c, 0, |mk| {
            mk.a = 0;
            mk.b = 3;
            mk.src = 1;
            mk.alu = 9; // R & S
            mk.cha = 7;
        });
        c.step();
        assert_eq!(c.registers.get(RegId::Alu), 0x0F00);
        let rfi = c.registers.get(RegId::Rfi);
        assert_eq!(rfi & 1, 0, "C сброшен для логических");
        assert_eq!((rfi >> 11) & 1, 0, "V сброшен для логических");
    }

    #[test]
    fn subtract_borrows_like_csharp() {
        // (uint)(0 - 0 - 1 + 0) = 0xFFFFFFFF => ALU = 0xFFFF, C = бит16 = 1
        let mut c = cpu();
        with_mk(&mut c, 0, |mk| {
            mk.src = 0;
            mk.alu = 1; // S - R - 1 + C0
            mk.ccx = 0;
            mk.cha = 7;
        });
        c.step();
        assert_eq!(c.registers.get(RegId::Alu), 0xFFFF);
        assert_eq!(c.registers.get(RegId::Rfi) & 1, 1, "C из бита 16");
    }

    #[test]
    fn shift_arithmetic_right() {
        let mut c = cpu();
        c.registers.set(RegId::Bx, 0x8000);
        with_mk(&mut c, 0, |mk| {
            mk.b = 3; // RGB = BX
            mk.src = 1;
            mk.alu = 8; // S (передача RGB)
            mk.ccx = 0;
            mk.sh = 1; // АС вправо
            mk.n = 4;
            mk.cha = 7;
        });
        c.step();
        // ALU=0x8000, АС вправо на 4 => 0xF800
        assert_eq!(c.registers.get(RegId::Sda), 0xF800);
    }

    #[test]
    fn shift_left_through_q() {
        let mut c = cpu();
        c.registers.set(RegId::Bx, 0x8000);
        c.registers.set(RegId::Rgq, 0xFFFF);
        with_mk(&mut c, 0, |mk| {
            mk.b = 3;
            mk.src = 1;
            mk.alu = 8; // S = RGB
            mk.sh = 0xA; // ЛС АЛУ, RGQ влево
            mk.n = 1;
            mk.cha = 7;
        });
        c.step();
        // ALU = 0x8000; SDA = 0x8000<<1 | RGQ>>15 = 0x0000 | 0x0001 = 0x0001
        assert_eq!(c.registers.get(RegId::Sda), 0x0001);
        // RGQ = 0xFFFF << 1 = 0xFFFE
        assert_eq!(c.registers.get(RegId::Rgq), 0xFFFE);
    }

    #[test]
    fn sign_extend_byte() {
        let mut c = cpu();
        with_mk(&mut c, 0, |mk| {
            mk.b = 3;
            mk.src = 1;
            mk.alu = 8; // S = RGB
            mk.sh = 0xE; // расширение знака
            mk.cha = 7;
        });
        c.registers.set(RegId::Bx, 0x00F0);
        c.step();
        assert_eq!(c.registers.get(RegId::Sda), 0xFFF0);

        c.registers.set(RegId::Bx, 0x007F);
        c.registers.set(RegId::Cmk, 0); // cha=7 инкрементировал CMK — возвращаем
        c.step();
        assert_eq!(c.registers.get(RegId::Sda), 0x007F);
    }

    #[test]
    fn memory_byte_roundtrip() {
        let mut c = cpu();
        // МК0: RGW = SDA (константа через SRC=CONST, ALU=R+C0)
        with_mk(&mut c, 0, |mk| {
            mk.src = 5; // r=CONST, s=RGB
            mk.alu = 6; // R + C0
            mk.const_ = 0xAB;
            mk.wm = 1; // RGW = SDA
            mk.cha = 7;
        });
        // МК1: ARAM = SDA
        with_mk(&mut c, 1, |mk| {
            mk.src = 5;
            mk.alu = 6;
            mk.const_ = 0x123;
            mk.wm = 2; // ARAM = SDA
            mk.cha = 7;
        });
        // МК2: запись байта
        with_mk(&mut c, 2, |mk| {
            mk.mem = 6;
            mk.cha = 0; // выборка следующей команды
        });
        c.registers.set(RegId::Cmk, 0);
        c.step();
        c.step();
        c.step();
        assert_eq!(c.ram.get(0x123), 0xAB);
        assert_eq!(c.registers.get(RegId::Cmk), 0); // после CHA=0

        // МК0 перепрограммируем на чтение байта: RGR = mem[ARAM]
        c.restart();
        with_mk(&mut c, 0, |mk| {
            mk.mem = 4;
            mk.cha = 7;
        });
        c.step();
        assert_eq!(c.registers.get(RegId::Rgr), 0xAB);
    }

    #[test]
    fn memory_out_of_bounds_event() {
        let mut c = cpu();
        c.registers.set(RegId::Aram, 0xFFFF);
        with_mk(&mut c, 0, |mk| {
            mk.mem = 4;
            mk.cha = 7;
        });
        let report = c.step();
        assert!(report
            .events
            .contains(&CpuEvent::MemoryReadOutOfBounds { address: 0xFFFF, word: false }));
    }

    #[test]
    fn jmap_decode() {
        let mut c = cpu();
        c.table.rows[0] = crate::translation::TableRow::new(0x20, "1000000000000000");
        c.registers.set(RegId::Rgk, 0x8000);
        with_mk(&mut c, 0, |mk| {
            mk.cha = 2; // JMAP
        });
        c.step();
        assert_eq!(c.registers.get(RegId::Cmk), 0x20);
    }

    #[test]
    fn jmap_decode_failed() {
        let mut c = cpu();
        c.table.reset();
        c.registers.set(RegId::Rgk, 0xFFFF);
        with_mk(&mut c, 0, |mk| {
            mk.cha = 2;
        });
        let report = c.step();
        assert!(report.events.contains(&CpuEvent::DecodeFailed));
        assert_eq!(c.registers.get(RegId::Cmk), 0);
        assert_eq!(c.state, ExecutionState::ReadyToStart); // состояние не меняется внутри execute
    }

    #[test]
    fn cjs_crtn_stack() {
        let mut c = cpu();
        // МК0: CJS безусловно (JFI=4), переход на 5, push 1
        with_mk(&mut c, 0, |mk| {
            mk.cha = 1; // CJS
            mk.jfi = 4; // безусловный
            mk.const_ = 5;
        });
        // МК5: CRTN безусловно
        with_mk(&mut c, 5, |mk| {
            mk.cha = 5; // CRTN
            mk.jfi = 4;
        });
        c.registers.set(RegId::Cmk, 0);
        c.step();
        assert_eq!(c.registers.get(RegId::Cmk), 5);
        assert_eq!(c.stack, vec![1]);
        c.step();
        assert_eq!(c.registers.get(RegId::Cmk), 1);
        assert!(c.stack.is_empty());
    }

    #[test]
    fn crtn_empty_stack_stops() {
        let mut c = cpu();
        with_mk(&mut c, 0, |mk| {
            mk.cha = 5;
            mk.jfi = 4;
        });
        let report = c.step();
        assert!(report.events.contains(&CpuEvent::StackEmpty));
        assert_eq!(c.state, ExecutionState::Stopped);
    }

    #[test]
    fn rpct_loop() {
        let mut c = cpu();
        // МК0: LDCT RACT=3
        with_mk(&mut c, 0, |mk| {
            mk.cha = 6;
            mk.const_ = 3;
        });
        // МК1: RPCT: пока RACT != 0 -> на 1
        with_mk(&mut c, 1, |mk| {
            mk.cha = 4;
            mk.const_ = 1;
        });
        // МК2: останов
        with_mk(&mut c, 2, |mk| {
            mk.jfi = 5;
        });
        c.registers.set(RegId::Cmk, 0);
        c.step(); // LDCT: RACT=3, CMK=1
        c.step(); // RPCT: RACT=2, CMK=1
        c.step(); // RPCT: RACT=1, CMK=1
        c.step(); // RPCT: RACT=0, CMK=1
        assert_eq!(c.registers.get(RegId::Ract), 0);
        c.step(); // RPCT: RACT=0 -> CMK++ = 2
        c.step(); // JFI=5 -> Stopped
        assert_eq!(c.state, ExecutionState::Stopped);
    }

    #[test]
    fn conditional_jump_z() {
        let mut c = cpu();
        // RFI.Z = 1 -> переход по CC=1 (JZ)
        c.registers.set(RegId::Rfi, 0x40);
        with_mk(&mut c, 0, |mk| {
            mk.cha = 3; // CJP
            mk.cc = 1; // JZ
            mk.jfi = 0; // RFI без инверсии
            mk.const_ = 0x2A;
        });
        c.step();
        assert_eq!(c.registers.get(RegId::Cmk), 0x2A);
    }

    #[test]
    fn conditional_jump_inverted() {
        let mut c = cpu();
        c.registers.set(RegId::Rfi, 0x40); // Z=1
        with_mk(&mut c, 0, |mk| {
            mk.cha = 3;
            mk.cc = 1;
            mk.jfi = 1; // инверсия
            mk.const_ = 0x2A;
        });
        c.step();
        // Z=1, инверсия -> не переходим, CMK++
        assert_eq!(c.registers.get(RegId::Cmk), 1);
    }

    #[test]
    fn conditional_jump_uses_rfd() {
        let mut c = cpu();
        c.registers.set(RegId::Rfi, 0);
        c.registers.set(RegId::Rfd, 0x40); // Z=1 в сохранённых
        with_mk(&mut c, 0, |mk| {
            mk.cha = 3;
            mk.cc = 1;
            mk.jfi = 2; // RFD
            mk.const_ = 0x2A;
        });
        c.step();
        assert_eq!(c.registers.get(RegId::Cmk), 0x2A);
    }

    #[test]
    fn halt_on_jfi5() {
        let mut c = cpu();
        with_mk(&mut c, 0, |mk| {
            mk.jfi = 5;
        });
        c.step();
        assert_eq!(c.state, ExecutionState::Stopped);
    }

    #[test]
    fn stop_at_mpm_end() {
        let mut c = cpu();
        c.registers.set(RegId::Cmk, MPM_SIZE as u16);
        let report = c.step();
        assert!(report.executed.is_none());
        assert_eq!(c.state, ExecutionState::Stopped);
    }

    #[test]
    fn dst_writes_to_register_from_rgk() {
        let mut c = cpu();
        // RGK: reg1 (биты 10:8) = 3 -> BX
        c.registers.set(RegId::Rgk, 0b0000_0011_0000_0000);
        c.registers.set(RegId::Ax, 0x1234);
        c.registers.set(RegId::Bx, 0x5678);
        with_mk(&mut c, 0, |mk| {
            mk.src = 1; // RGA, RGB
            mk.alu = 3; // R + S
            mk.dst = 4; // РЗУ = SDA
            mk.mb = 1; // приёмник из RGK.reg1
            mk.cha = 7;
        });
        c.step();
        assert_eq!(c.registers.get(RegId::Bx), 0x68AC);
    }

    #[test]
    fn dst_byte_insertions() {
        let mut c = cpu();
        c.registers.set(RegId::Rgr, 0x00AB);
        c.registers.set(RegId::Ax, 0xCD00);
        with_mk(&mut c, 0, |mk| {
            mk.dst = 2; // РЗУ(ст.) = RGR(мл.)<<8: dst = (dst & 0xFF) | RGR<<8
            mk.a = 0; // AX
            mk.mb = 0;
            mk.b = 0;
            mk.cha = 7;
            mk.alu = 6; // R + C0 (не важно)
            mk.src = 1;
        });
        c.step();
        assert_eq!(c.registers.get(RegId::Ax), 0xAB00);

        c.registers.set(RegId::Ax, 0x00CD);
        c.registers.set(RegId::Rgr, 0xAB00);
        with_mk(&mut c, 1, |mk| {
            mk.dst = 3; // РЗУ(мл.) = RGR(ст.)>>8
            mk.b = 0;
            mk.cha = 7;
        });
        c.registers.set(RegId::Cmk, 1);
        c.step();
        assert_eq!(c.registers.get(RegId::Ax), 0x00AB);
    }

    #[test]
    fn restart_clears_state() {
        let mut c = cpu();
        c.registers.set(RegId::Cmk, 10);
        c.registers.set(RegId::Rfi, 0x40);
        c.stack.push(3);
        c.restart();
        assert_eq!(c.registers.get(RegId::Cmk), 0);
        assert_eq!(c.registers.get(RegId::Rfi), 0);
        assert!(c.stack.is_empty());
        assert_eq!(c.state, ExecutionState::ReadyToStart);
        assert_eq!(c.trace.len(), 1);
    }

    #[test]
    fn step_instruction_runs_until_fetch() {
        let mut c = cpu();
        // МК0..MK2: CONT, MK3: CMK=0 (выборка)
        for i in 0..3 {
            with_mk(&mut c, i, |mk| {
                mk.cha = 7;
            });
        }
        with_mk(&mut c, 3, |mk| {
            mk.cha = 0;
        });
        c.registers.set(RegId::Cmk, 0);
        let report = c.step_instruction();
        assert!(report.executed.is_some());
        assert_eq!(c.registers.get(RegId::Cmk), 0); // вернулись к выборке
        assert_eq!(c.state, ExecutionState::Paused);
        assert_eq!(c.trace.len(), 5); // начальный + 4 шага
    }

    #[test]
    fn field_set_masks() {
        let mut mk = MicroCommand::new(0);
        mk.set(Field::Ma, 7);
        assert_eq!(mk.ma, 3);
        mk.set(Field::Mem, 9);
        assert_eq!(mk.mem, 1);
        mk.set(Field::Const, 0x2345);
        assert_eq!(mk.const_, 0x2345);
    }

    #[test]
    fn trace_row_shape() {
        let mut c = cpu();
        c.step();
        assert_eq!(c.trace.len(), 2);
        assert_eq!(c.trace[0].len(), 27);
        assert_eq!(c.trace[0][0], 0); // CMK
    }
}
