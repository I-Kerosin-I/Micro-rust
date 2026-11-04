//! Вкладка «Дешифратор»: таблица преобразования адресов (20 строк).

use crate::theme::Palette;
use egui::{RichText, TextEdit};
use micro_core::CpuState;

#[derive(Debug, Default)]
pub struct TtaViewState {
    addr_bufs: Vec<String>,
    opcode_bufs: Vec<String>,
    focused: Vec<bool>,
}

impl TtaViewState {
    fn ensure(&mut self) {
        let n = micro_core::TABLE_ROWS;
        if self.addr_bufs.len() != n {
            self.addr_bufs = vec![String::new(); n];
            self.opcode_bufs = vec![String::new(); n];
            self.focused = vec![false; n * 2];
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Load,
    Save,
}

pub fn show(
    ui: &mut egui::Ui,
    p: &Palette,
    state: &mut CpuState,
    view: &mut TtaViewState,
) -> Option<Action> {
    let mut action = None;
    view.ensure();

    // Панель действий (сверху)
    crate::widgets::toolbar(ui, |ui| {
        if ui.button("Загрузить…").clicked() {
            action = Some(Action::Load);
        }
        if ui.button("Сохранить…").clicked() {
            action = Some(Action::Save);
        }
    });
    crate::widgets::hint_line(
        ui,
        p,
        "0/1 — фиксированные биты, X — любой бит · дешифрация идёт по первой совпавшей строке",
    );
    ui.add_space(2.0);

    egui::Frame::new()
        .fill(p.panel_bg)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            let mut table = egui_extras::TableBuilder::new(ui)
                .id_salt("tta_table")
                .striped(true)
                .vscroll(false)
                .min_scrolled_height(0.0)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center));
            table = table.column(egui_extras::Column::exact(40.0).resizable(false).clip(false));
            table = table.column(egui_extras::Column::exact(80.0).resizable(false).clip(false));
            table = table.column(egui_extras::Column::remainder().resizable(false).clip(false));

            table.header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label(RichText::new("#").size(11.0).color(p.text_dim));
                });
                header.col(|ui| {
                    ui.label(RichText::new("Начальный адрес").size(11.0).color(p.text_dim));
                });
                header.col(|ui| {
                    ui.label(
                        RichText::new("Код операции (0/1/X — 16 символов)")
                            .size(11.0)
                            .color(p.text_dim),
                    );
                });
            })
            .body(|mut body| {
                for i in 0..micro_core::TABLE_ROWS {
                    body.row(24.0, |mut row| {
                        row.col(|ui| {
                            ui.label(RichText::new(format!("{}", i + 1)).color(p.text_dim).size(12.0));
                        });
                        row.col(|ui| {
                            // Адрес
                            let value = state.table.rows[i].address as u16;
                            if !view.focused[i * 2] {
                                view.addr_bufs[i] = format!("{:02X}", value);
                            }
                            let resp = ui.add(
                                TextEdit::singleline(&mut view.addr_bufs[i])
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(44.0)
                                    .margin(egui::Margin::symmetric(4, 2)),
                            );
                            let was = view.focused[i * 2];
                            view.focused[i * 2] = resp.has_focus();
                            if resp.changed() {
                                let s = crate::widgets::normalize_hex_string(&view.addr_bufs[i], 2);
                                view.addr_bufs[i] = s.clone();
                                if let Ok(v) = u16::from_str_radix(&s, 16) {
                                    state.table.rows[i].address = v as u8;
                                    state.tta_modified = true;
                                }
                            }
                            if was && !view.focused[i * 2] {
                                view.addr_bufs[i] = format!("{:02X}", value);
                            }
                        });
                        row.col(|ui| {
                            // Опкод (0/1/X)
                            let opcode = state.table.rows[i].opcode.clone();
                            if !view.focused[i * 2 + 1] {
                                view.opcode_bufs[i] = opcode;
                            }
                            let resp = ui.add(
                                TextEdit::singleline(&mut view.opcode_bufs[i])
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(170.0)
                                    .margin(egui::Margin::symmetric(4, 2)),
                            );
                            let was = view.focused[i * 2 + 1];
                            view.focused[i * 2 + 1] = resp.has_focus();
                            if resp.changed() {
                                let s: String = view.opcode_bufs[i]
                                    .chars()
                                    .filter_map(crate::widgets::normalize_opcode_char)
                                    .take(16)
                                    .collect();
                                view.opcode_bufs[i] = s.clone();
                                if s.len() == 16 {
                                    state.table.rows[i].opcode = s;
                                    state.tta_modified = true;
                                }
                            }
                            if was && !view.focused[i * 2 + 1] {
                                view.opcode_bufs[i] = state.table.rows[i].opcode.clone();
                            }
                            // Индикатор корректности
                            let valid = view.opcode_bufs[i].len() == 16;
                            if !valid {
                                ui.painter().circle_filled(
                                    resp.rect.right_center() + egui::vec2(-6.0, 0.0),
                                    3.0,
                                    p.warning,
                                );
                            }
                        });
                    });
                }
            });
        });

    action
}
