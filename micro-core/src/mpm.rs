//! Память микрокоманд (МПП): 64 ячейки.

use crate::microcommand::MicroCommand;

pub const MPM_SIZE: usize = 64;

/// Память микрокоманд.
#[derive(Debug, Clone)]
pub struct MicroProgramMemory {
    commands: Vec<MicroCommand>,
}

impl Default for MicroProgramMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl MicroProgramMemory {
    pub fn new() -> Self {
        Self {
            commands: (0..MPM_SIZE as u8).map(MicroCommand::new).collect(),
        }
    }

    pub fn size(&self) -> usize {
        MPM_SIZE
    }

    #[inline]
    pub fn get(&self, index: usize) -> &MicroCommand {
        &self.commands[index]
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> &mut MicroCommand {
        &mut self.commands[index]
    }

    #[inline]
    pub fn set(&mut self, index: usize, cmd: MicroCommand) {
        self.commands[index] = cmd;
    }

    /// Вставка пустой МК по адресу со сдвигом последующих вправо
    /// (последняя теряется), как в оригинале `InsertMicrocommand`.
    pub fn insert(&mut self, address: usize) {
        for i in (address + 1..MPM_SIZE).rev() {
            let mut prev = self.commands[i - 1].clone();
            prev.address = i as u8;
            self.commands[i] = prev;
        }
        self.commands[address] = MicroCommand::new(address as u8);
    }

    /// Удаление МК по адресу со сдвигом последующих влево
    /// (в конец записывается пустая), как в оригинале `DeleteMicrocommand`.
    pub fn remove(&mut self, address: usize) {
        for i in address..MPM_SIZE - 1 {
            let mut next = self.commands[i + 1].clone();
            next.address = i as u8;
            self.commands[i] = next;
        }
        self.commands[MPM_SIZE - 1] = MicroCommand::new((MPM_SIZE - 1) as u8);
    }

    pub fn iter(&self) -> impl Iterator<Item = &MicroCommand> {
        self.commands.iter()
    }

    /// Заменить всё содержимое (загрузка из файла). Размер приводится к 64:
    /// если в файле меньше — остаток заполняется пустыми МК, больше — лишнее отбрасывается.
    pub fn load_from(&mut self, commands: Vec<MicroCommand>) {
        for (i, cmd) in self.commands.iter_mut().enumerate() {
            if i < commands.len() {
                *cmd = commands[i].clone();
                cmd.address = i as u8;
            } else {
                *cmd = MicroCommand::new(i as u8);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_shifts_right() {
        let mut mpm = MicroProgramMemory::new();
        mpm.get_mut(0).a = 1;
        mpm.get_mut(1).a = 2;
        mpm.insert(0);
        assert_eq!(mpm.get(0).a, 0); // новая пустая
        assert_eq!(mpm.get(1).a, 1);
        assert_eq!(mpm.get(2).a, 2);
        assert_eq!(mpm.get(1).address, 1);
        assert_eq!(mpm.get(2).address, 2);
    }

    #[test]
    fn remove_shifts_left() {
        let mut mpm = MicroProgramMemory::new();
        mpm.get_mut(0).a = 1;
        mpm.get_mut(1).a = 2;
        mpm.remove(0);
        assert_eq!(mpm.get(0).a, 2);
        assert_eq!(mpm.get(MPM_SIZE - 1).a, 0);
        assert_eq!(mpm.get(0).address, 0);
    }
}
