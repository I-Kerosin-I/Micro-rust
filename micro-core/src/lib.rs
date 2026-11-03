//! micro-core — ядро симулятора микропрограммируемого процессора Micro.
//!
//! Не зависит от GUI. Содержит модель процессора (регистры, память
//! микрокоманд, ОЗУ, дешифратор), исполнение микрокоманд и работу с файлами
//! данных оригинальной программы (.MEM/.XMEM/.RAM/.RG/.TTA).

pub mod cpu;
pub mod formats;
pub mod microcommand;
pub mod mpm;
pub mod ram;
pub mod register;
pub mod trace;
pub mod translation;

pub use cpu::{CpuEvent, CpuState, ExecutionState, StepReport};
pub use microcommand::{field_options, Field, MicroCommand, REGISTER_NAMES};
pub use mpm::{MicroProgramMemory, MPM_SIZE};
pub use ram::{RamMemory, RAM_SIZE};
pub use register::{RegId, RegisterMemory};
pub use translation::{AddressTranslationTable, TableRow, TABLE_ROWS};
