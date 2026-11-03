//! Вкладка «Работа»: схема процессора с значениями регистров,
//! выполненная/следующая микрокоманда, флаги, стек.

use crate::theme::Palette;
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, ScrollArea, Sense, Vec2};
use micro_core::{CpuState, ExecutionState, Field, RegId};

/// Локальное состояние вкладки.
#[derive(Debug, Default)]
pub struct CpuViewState {
    pub executed: Option<micro_core::MicroCommand>,
}

/// Размер отображения схемы в оригинале (XAML-канва).
const DISPLAY_W: f32 = 1000.0;
const DISPLAY_H: f32 = 618.4;

/// Заголовки колонок таблицы микрокоманд.
const MK_COLUMNS: [(&str, Field); 17] = [
    ("A", Field::A),
    ("B", Field::B),
    ("MA", Field::Ma),
    ("MB", Field::Mb),
    ("MEM", Field::Mem),
    ("SRC", Field::Src),
    ("SH", Field::Sh),
    ("N", Field::N),
    ("ALU", Field::Alu),
    ("CCX", Field::Ccx),
    ("F", Field::F),
    ("DST", Field::Dst),
    ("WM", Field::Wm),
    ("JFI", Field::Jfi),
    ("CC", Field::Cc),
    ("CHA", Field::Cha),
    ("CONST", Field::Const),
];

pub fn show(
    ui: &mut egui::Ui,
    p: &Palette,
    state: &mut CpuState,
    view: &mut CpuViewState,
    texture_dark: &egui::TextureHandle,
    texture_light: &egui::TextureHandle,
    is_dark: bool,
) {
    // --- Таблица «Выполнена / Следующая» (вплотную к шапке и меню) ---
    next_mk_panel(ui, p, state, view);
    ui.add_space(8.0);

    // --- Схема с наложениями ---
    ScrollArea::both().auto_shrink(false).show(ui, |ui| {
        let texture = if is_dark { texture_dark } else { texture_light };
        let avail = ui.available_size();
        let scale = (avail.x / DISPLAY_W).min(avail.y / DISPLAY_H).min(1.2);
        let size = Vec2::new(DISPLAY_W * scale, DISPLAY_H * scale);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let image = egui::Image::new((texture.id(), size));
        image.paint_at(ui, rect);

        paint_overlays(ui, p, state, rect);
    });
}

fn fmt_reg(state: &CpuState, reg: RegId) -> String {
    format!("{:04X}", state.registers.get(reg))
}

fn fmt_mk_row(mk: Option<&micro_core::MicroCommand>, cmk: u16) -> Vec<String> {
    let mut cells = vec![match mk {
        Some(m) => format!("{:02X}", m.address),
        None => format!("{:02X}", cmk),
    }];
    for (_, field) in MK_COLUMNS.iter() {
        cells.push(match mk {
            Some(m) => {
                let v = m.get(*field);
                if *field == Field::Const {
                    format!("{:04X}", v)
                } else {
                    format!("{:01X}", v)
                }
            }
            None => "—".to_string(),
        });
    }
    cells.push(
        mk.and_then(|m| {
            if m.comment.is_empty() {
                None
            } else {
                Some(m.comment.clone())
            }
        })
        .unwrap_or_default(),
    );
    cells
}

fn next_mk_panel(ui: &mut egui::Ui, p: &Palette, state: &CpuState, view: &mut CpuViewState) {
    let cmk = state.registers.get(RegId::Cmk) as usize;
    let next = if cmk < state.mpm.size() {
        Some(state.mpm.get(cmk).clone())
    } else {
        None
    };

    let rows = [
        ("Выполнена", view.executed.clone()),
        ("Следующая", next),
    ];

    egui::Frame::new()
        .fill(p.card_bg)
        .inner_margin(egui::Margin {
            left: 10,
            right: 10,
            top: 5,
            bottom: 7,
        })
        .show(ui, |ui| {
            let mut table = egui_extras::TableBuilder::new(ui)
                .id_salt("next_mk_table")
                .striped(false)
                .cell_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight))
                .vscroll(false)
                .min_scrolled_height(0.0);
            table = table.column(egui_extras::Column::exact(74.0).resizable(false).clip(false));
            for (_, field) in MK_COLUMNS.iter() {
                let w = if *field == Field::Const { 46.0 } else { 34.0 };
                table = table.column(egui_extras::Column::exact(w).resizable(false).clip(false));
            }
            table = table.column(egui_extras::Column::remainder().resizable(false).clip(false));
            table
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label(RichText::new("МК").size(11.0).color(p.text_dim));
                    });
                    for (name, _) in MK_COLUMNS.iter() {
                        header.col(|ui| {
                            ui.label(RichText::new(*name).size(11.0).color(p.text_dim));
                        });
                    }
                    header.col(|ui| {
                        ui.label(RichText::new("Комментарий").size(11.0).color(p.text_dim));
                    });
                })
                .body(|mut body| {
                    for (label, mk) in rows {
                        let cells = fmt_mk_row(mk.as_ref(), state.registers.get(RegId::Cmk));
                        let is_executed = label == "Выполнена";
                        body.row(24.0, |mut row| {
                            row.col(|ui| {
                                ui.label(
                                    RichText::new(label)
                                        .size(12.0)
                                        .color(if is_executed { p.accent } else { p.text_muted }),
                                );
                            });
                            for (i, cell) in cells.iter().enumerate() {
                                row.col(|ui| {
                                    let text = if i == cells.len() - 1 {
                                        RichText::new(cell.clone()).size(12.5)
                                    } else {
                                        mono_size_colored(cell.clone(), 13.0, p.text)
                                    };
                                    ui.label(text);
                                });
                            }
                        });
                    }
                });
        });
}

fn mono_size_colored(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text.into()).font(FontId::monospace(size)).color(color)
}

/// Наложение значений регистров на схему.
///
/// Координаты заданы в системе XAML-канвы оригинала (изображение 1000×618.4).
struct Overlay {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &'static str,
    reg: Option<RegId>,
    font: f32,
}

const OVERLAYS: &[Overlay] = &[
    Overlay { x: 34.0, y: 218.0, w: 89.0, h: 22.0, label: "ARAM ", reg: Some(RegId::Aram), font: 12.5 },
    Overlay { x: 188.0, y: 202.0, w: 89.0, h: 22.0, label: "RGA  ", reg: Some(RegId::Rga), font: 12.5 },
    Overlay { x: 324.0, y: 202.0, w: 89.0, h: 22.0, label: "RGB  ", reg: Some(RegId::Rgb), font: 12.5 },
    Overlay { x: 257.0, y: 414.0, w: 89.0, h: 22.0, label: "SDA  ", reg: Some(RegId::Sda), font: 12.5 },
    Overlay { x: 422.0, y: 306.0, w: 88.0, h: 22.0, label: "RGQ  ", reg: Some(RegId::Rgq), font: 12.5 },
    Overlay { x: 36.0, y: 301.0, w: 88.0, h: 22.0, label: "RGR  ", reg: Some(RegId::Rgr), font: 12.5 },
    Overlay { x: 36.0, y: 370.0, w: 88.0, h: 22.0, label: "RGW  ", reg: Some(RegId::Rgw), font: 12.5 },
    Overlay { x: 685.0, y: 381.0, w: 89.0, h: 23.0, label: "CMK  ", reg: Some(RegId::Cmk), font: 12.5 },
    Overlay { x: 591.0, y: 57.0, w: 88.0, h: 22.0, label: "RACT ", reg: Some(RegId::Ract), font: 12.5 },
];

fn paint_overlays(ui: &mut egui::Ui, p: &Palette, state: &CpuState, rect: Rect) {
    let painter = ui.painter();
    let sx = rect.width() / DISPLAY_W;
    let sy = rect.height() / DISPLAY_H;

    let norm = |x: f32, y: f32, w: f32, h: f32| {
        Rect::from_min_size(
            Pos2::new(rect.left() + x * sx, rect.top() + y * sy),
            Vec2::new(w * sx, h * sy),
        )
    };

    // Регистр-значение с подсветкой изменений
    let value_box = |painter: &egui::Painter, r: Rect, text: String, changed: bool, font: f32| {
        painter.rect_filled(r, 3.0, if changed { p.highlight } else { p.card_bg });
        painter.rect_stroke(r, 3.0, egui::Stroke::new(1.0, if changed { p.success } else { p.border }), egui::StrokeKind::Inside);
        painter.text(r.center(), Align2::CENTER_CENTER, text, FontId::monospace(font * sx.min(sy).max(0.8)), p.text);
    };

    // Сетка РОНов 4×4 (как UniformGrid оригинала)
    let grid = norm(170.0, 75.0, 389.0, 100.0);
    let cell_w = grid.width() / 4.0;
    let cell_h = grid.height() / 4.0;
    let grid_regs: [[RegId; 4]; 4] = [
        [RegId::Ax, RegId::Cx, RegId::Dx, RegId::Bx],
        [RegId::Sp, RegId::Bp, RegId::Si, RegId::Di],
        [RegId::Cs, RegId::Ds, RegId::Ss, RegId::Es],
        [RegId::Ip, RegId::Psw, RegId::Rgk, RegId::Rw],
    ];
    let grid_labels: [[&str; 4]; 4] = [
        ["AX   ", "CX   ", "DX   ", "BX   "],
        ["SP   ", "BP   ", "SI   ", "DI   "],
        ["CS   ", "DS   ", "SS   ", "ES   "],
        ["IP   ", "PSW  ", "RGK  ", "RW   "],
    ];
    for (row, regs) in grid_regs.iter().enumerate() {
        for (col, reg) in regs.iter().enumerate() {
            let r = Rect::from_min_size(
                Pos2::new(grid.left() + col as f32 * cell_w, grid.top() + row as f32 * cell_h),
                Vec2::new(cell_w - 2.0, cell_h - 1.0),
            );
            let text = format!("{}{}", grid_labels[row][col], fmt_reg(state, *reg));
            value_box(painter, r, text, state.registers.is_changed(*reg), 11.5);
        }
    }

    // Отдельные поля
    for o in OVERLAYS {
        let r = norm(o.x, o.y, o.w, o.h);
        let reg = o.reg.unwrap();
        let text = format!("{}{}", o.label, fmt_reg(state, reg));
        value_box(painter, r, text, state.registers.is_changed(reg), o.font);
    }

    // ALU — крупный блок
    let alu_r = norm(202.0, 333.0, 198.0, 44.0);
    value_box(
        painter,
        alu_r,
        format!("ALU  {}", fmt_reg(state, RegId::Alu)),
        state.registers.is_changed(RegId::Alu),
        16.0,
    );

    // RGK в двоичном виде (5-3-2-3-3 группы, как в оригинале)
    let rgk = state.registers.get(RegId::Rgk);
    let bin = format!(
        "{:016b}",
        rgk
    );
    let bin_grouped = format!("{} {} {} {} {}", &bin[0..5], &bin[5..8], &bin[8..10], &bin[10..13], &bin[13..16]);
    let rgk_r = norm(727.0, 432.0, 170.0, 22.0);
    value_box(
        painter,
        rgk_r,
        bin_grouped,
        state.registers.is_changed(RegId::Rgk),
        12.0,
    );

    // STP — глубина стека
    let stp_r = norm(685.0, 111.0, 89.0, 23.0);
    value_box(
        painter,
        stp_r,
        format!("STP  {:04X}", state.stack.len()),
        false,
        12.5,
    );

    // STOP-индикатор
    let stop_r = norm(855.0, 111.0, 89.0, 23.0);
    let stopped = state.state == ExecutionState::Stopped;
    painter.rect_filled(stop_r, 3.0, if stopped { p.danger.gamma_multiply(0.25) } else { p.card_bg });
    painter.rect_stroke(
        stop_r,
        3.0,
        egui::Stroke::new(1.0, if stopped { p.danger } else { p.border }),
        egui::StrokeKind::Inside,
    );
    painter.text(
        stop_r.center(),
        Align2::CENTER_CENTER,
        "STOP",
        FontId::proportional(13.0),
        if stopped { p.danger } else { p.text_dim },
    );

    // Стек адресов возврата
    let stack_r = norm(685.0, 134.0, 88.0, 192.0);
    painter.rect_filled(stack_r, 3.0, p.card_bg);
    painter.rect_stroke(stack_r, 3.0, egui::Stroke::new(1.0, p.border), egui::StrokeKind::Inside);
    let font = FontId::monospace(11.5 * sx.max(0.8));
    let line_h = 16.0 * sy.max(0.8);
    let visible_lines = ((stack_r.height() - 8.0) / line_h).floor() as usize;
    let start = state.stack.len().saturating_sub(visible_lines);
    for (i, addr) in state.stack.iter().enumerate().skip(start).take(visible_lines) {
        let y = stack_r.top() + 4.0 + (i - start) as f32 * line_h;
        painter.text(
            Pos2::new(stack_r.left() + 8.0, y + line_h * 0.5),
            Align2::LEFT_CENTER,
            format!("{:04X}", addr),
            font.clone(),
            if i + 1 == state.stack.len() { p.accent } else { p.text_muted },
        );
    }

    // Таблица флагов RFI/RFD — текст внутри рамки, предусмотренной на схеме
    // (посадочное место измерено по PNG: канва (104.2, 412.3), размер 128.9×66.6).
    let flags_r = norm(104.2, 412.3, 128.9, 66.6);
    let col_x = |frac: f32| flags_r.left() + flags_r.width() * frac;
    let headers = ["", "N", "Z", "V", "C", "P"];
    let fracs = [0.16, 0.35, 0.52, 0.69, 0.86, 0.97];
    let header_font = FontId::proportional(10.0);
    let val_font = FontId::monospace(11.5);
    for (h, f) in headers.iter().zip(fracs.iter()) {
        painter.text(
            Pos2::new(col_x(*f), flags_r.top() + 13.0),
            Align2::CENTER_CENTER,
            *h,
            header_font.clone(),
            p.text_dim,
        );
    }
    for (row, reg) in [RegId::Rfi, RegId::Rfd].iter().enumerate() {
        let v = state.registers.get(*reg);
        let y = flags_r.top() + 35.0 + row as f32 * 20.0;
        painter.text(
            Pos2::new(col_x(fracs[0]), y),
            Align2::CENTER_CENTER,
            reg.name(),
            FontId::monospace(10.5),
            p.text_muted,
        );
        let bits = [(v >> 7) & 1, (v >> 6) & 1, (v >> 11) & 1, v & 1, (v >> 2) & 1];
        for (bit, f) in bits.iter().zip(fracs[1..].iter()) {
            painter.text(
                Pos2::new(col_x(*f), y),
                Align2::CENTER_CENTER,
                format!("{}", bit),
                val_font.clone(),
                if *bit == 1 { p.warning } else { p.text_dim },
            );
        }
    }
}
