//! Трасса исполнения: экспорт в CSV, совместимый с оригиналом
//! (разделитель `;`, UTF-8 с BOM, колонки как в `CpuViewModel.SaveTrace`).

use crate::cpu::TraceRow;

/// Заголовки CSV (идентичны оригиналу, включая регистр букв).
pub const CSV_HEADERS: [&str; 31] = [
    "CMK", "AX", "CX", "DX", "BX", "SP", "BP", "SI", "DI", "CS", "SS", "DS", "ES", "IP", "PSW",
    "RGK", "RW", "RGA", "RGB", "Alu", "Sda", "RGQ", "N", "Z", "V", "C", "P", "ARAM", "RGR", "RGW",
    "RACT",
];

/// Экспорт трассы в CSV-строку (UTF-8 с BOM, как в .NET `Encoding.UTF8`).
pub fn export_csv(trace: &[TraceRow]) -> String {
    let mut out = String::with_capacity(trace.len() * 130);
    out.push('\u{FEFF}'); // BOM
    out.push_str(&CSV_HEADERS.join(";"));
    out.push('\n');
    for row in trace {
        // row: CMK, AX..RGQ (индексы 1..22), RFI(22), ARAM..RACT (23..27)
        let mut cells: Vec<String> = Vec::with_capacity(31);
        cells.push(format!("{:02X}", row[0]));
        for v in &row[1..22] {
            cells.push(format!("{:04X}", v));
        }
        let rfi = row[22];
        cells.push(((rfi >> 7) & 1).to_string()); // N
        cells.push(((rfi >> 6) & 1).to_string()); // Z
        cells.push(((rfi >> 11) & 1).to_string()); // V
        cells.push((rfi & 1).to_string()); // C
        cells.push(((rfi >> 2) & 1).to_string()); // P
        for v in &row[23..27] {
            cells.push(format!("{:04X}", v));
        }
        out.push_str(&cells.join(";"));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_shape() {
        let trace: Vec<TraceRow> = vec![[0u16; 27], [1u16; 27]];
        let csv = export_csv(&trace);
        let mut lines = csv.lines();
        let header = lines.next().unwrap().trim_start_matches('\u{FEFF}');
        assert_eq!(
            header,
            "CMK;AX;CX;DX;BX;SP;BP;SI;DI;CS;SS;DS;ES;IP;PSW;RGK;RW;RGA;RGB;Alu;Sda;RGQ;N;Z;V;C;P;ARAM;RGR;RGW;RACT"
        );
        let row = lines.next().unwrap();
        let cells: Vec<&str> = row.split(';').collect();
        assert_eq!(cells.len(), 31);
        assert_eq!(cells[0], "00");
        assert_eq!(cells[1], "0000");
        assert_eq!(cells[22], "0"); // N
        assert_eq!(cells[30], "0000"); // RACT
        assert_eq!(lines.next(), Some("01;0001;0001;0001;0001;0001;0001;0001;0001;0001;0001;0001;0001;0001;0001;0001;0001;0001;0001;0001;0001;0001;0;0;0;1;0;0001;0001;0001;0001"));
    }
}
