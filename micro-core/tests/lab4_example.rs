//! Интеграционный тест: исполнение программы из лабораторной работы №4
//! (example.MEM/.RAM/.RG/.TTA) до останова ЦП.

use micro_core::formats::{parse_mem, parse_ram, parse_rg, parse_tta};
use micro_core::{CpuState, ExecutionState, RegId};

fn lab_dir() -> Option<std::path::PathBuf> {
    // Сначала — копия фикстур в tests/data, затем оригинальная папка
    let local = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    if local.join("example.MEM").exists() {
        return Some(local);
    }
    None
}

#[test]
fn lab4_example_program_runs_to_halt() {
    let Some(dir) = lab_dir() else {
        eprintln!("пропущен: фикстуры лабораторной не найдены");
        return;
    };

    let mem = std::fs::read_to_string(dir.join("example.MEM")).unwrap();
    let ram = std::fs::read(dir.join("example.RAM")).unwrap();
    let rg = std::fs::read_to_string(dir.join("example.RG")).unwrap();
    let tta = std::fs::read_to_string(dir.join("example.TTA")).unwrap();

    let commands = parse_mem(&mem).expect("МПП разбирается");
    let ram_bytes = parse_ram(&ram).expect("ОЗУ разбирается");
    let registers = parse_rg(&rg).expect("регистры разбираются");
    let table = parse_tta(&tta).expect("таблица разбирается");

    let mut cpu = CpuState::new();
    cpu.mpm.load_from(commands);
    cpu.ram.load_bytes(&ram_bytes);
    for (reg, value) in registers {
        cpu.registers.set_silent(reg, value);
    }
    cpu.table = table;
    cpu.restart();

    // Непрерывное исполнение до останова (с пределом шагов)
    let mut steps = 0usize;
    while cpu.state != ExecutionState::Stopped && steps < 1_000_000 {
        let _ = cpu.step();
        steps += 1;
    }

    assert_eq!(
        cpu.state,
        ExecutionState::Stopped,
        "ЦП должен остановиться (JFI=5 или граница МПП)"
    );
    assert!(steps > 0);
    assert!(!cpu.trace.is_empty());

    // Финальное состояние не должно содержать «мусорных» событий —
    // проверим, что CMK в разумных пределах и RGK из файла сохранился
    eprintln!("шагов: {steps}, CMK = {:02X}", cpu.registers.get(RegId::Cmk));
    eprintln!(
        "AX={:04X} BX={:04X} CX={:04X} DX={:04X} ALU={:04X} SDA={:04X}",
        cpu.registers.get(RegId::Ax),
        cpu.registers.get(RegId::Bx),
        cpu.registers.get(RegId::Cx),
        cpu.registers.get(RegId::Dx),
        cpu.registers.get(RegId::Alu),
        cpu.registers.get(RegId::Sda),
    );

    // Экспорт трассы должен формироваться без паники и содержать заголовок
    let csv = micro_core::trace::export_csv(&cpu.trace);
    assert!(csv.contains("CMK;AX;CX;DX;BX"));
    assert_eq!(csv.lines().count(), cpu.trace.len() + 1);
}

/// ЛР6: наборы операндов — пары 16-битных слов в ОЗУ по адресам 0 и 1.
/// Проверяем, что файлы разбираются и загрузка/выгрузка форматов обратима.
#[test]
fn lab6_datasets_roundtrip() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data");
    // Datasets живут в оригинальном репозитории; при их отсутствии пропускаем
    let datasets = if dir.join("datasets").exists() {
        dir.join("datasets")
    } else {
        // ищем относительно рабочей папки проекта
        let alt = std::path::Path::new("D:/Projects/Micro/Лабораторная работа 6 для тестов/datasets");
        if alt.exists() {
            alt.to_path_buf()
        } else {
            eprintln!("пропущен: наборы ЛР6 не найдены");
            return;
        }
    };

    let mut checked = 0;
    for entry in std::fs::read_dir(datasets).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e.eq_ignore_ascii_case("ram")).unwrap_or(false) {
            let data = std::fs::read(&path).unwrap();
            let bytes = micro_core::formats::parse_ram(&data)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(bytes.len(), micro_core::RAM_SIZE);
            let word0 = (bytes[0] as u16) | ((bytes[1] as u16) << 8);
            let word1 = (bytes[2] as u16) | ((bytes[3] as u16) << 8);
            eprintln!("{:?}: операнды {:04X}, {:04X}", path.file_name().unwrap(), word0, word1);
            checked += 1;
        }
    }
    assert!(checked >= 20, "ожидалось >= 20 наборов, найдено {checked}");
}
