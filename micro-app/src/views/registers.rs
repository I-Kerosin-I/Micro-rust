//! Вкладка «Регистры»: редактор всех регистров процессора.

use crate::theme::Palette;
use crate::widgets::{muted_label, section_title, HexInput};
use egui::RichText;
use micro_core::{CpuState, RegId};

/// Состояние редактора: HEX-поля для каждого регистра.
#[derive(Debug, Default)]
pub struct RegistersViewState {
    edits: [HexInput; 31],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Load,
    Save,
    Reset,
}

pub fn show(
    ui: &mut egui::Ui,
    p: &Palette,
    state: &mut CpuState,
    view: &mut RegistersViewState,
) -> Option<Action> {
    let mut action = None;

    egui::Frame::new()
        .fill(p.panel_bg)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.columns(2, |cols| {
                // --- РОНы ---
                cols[0].vertical(|ui| {
                    card(ui, p, "РОНы", |ui| {
                        let order: [[RegId; 4]; 4] = [
                            [RegId::Ax, RegId::Cx, RegId::Dx, RegId::Bx],
                            [RegId::Sp, RegId::Bp, RegId::Si, RegId::Di],
                            [RegId::Cs, RegId::Ds, RegId::Ss, RegId::Es],
                            [RegId::Ip, RegId::Psw, RegId::Rgk, RegId::Rw],
                        ];
                        egui::Grid::new("ron_grid")
                            .num_columns(8)
                            .spacing([6.0, 4.0])
                            .show(ui, |ui| {
                                for row in order {
                                    for reg in row {
                                        ui.label(
                                            RichText::new(reg.name())
                                                .monospace()
                                                .size(12.0)
                                                .color(p.text_muted),
                                        );
                                        reg_edit(ui, p, state, view, reg);
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                });

                // --- Флаги + RGQ ---
                cols[1].vertical(|ui| {
                    card(ui, p, "Регистры флагов", |ui| {
                        egui::Grid::new("flags_grid")
                            .num_columns(2)
                            .spacing([6.0, 4.0])
                            .show(ui, |ui| {
                                for reg in [RegId::Rfi, RegId::Rfd] {
                                    ui.label(
                                        RichText::new(reg.name())
                                            .monospace()
                                            .size(12.0)
                                            .color(p.text_muted),
                                    );
                                    reg_edit(ui, p, state, view, reg);
                                    ui.end_row();
                                }
                            });
                    });
                    ui.add_space(6.0);
                    card(ui, p, "Рабочий регистр", |ui| {
                        egui::Grid::new("rgq_grid")
                            .num_columns(2)
                            .spacing([6.0, 4.0])
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(RegId::Rgq.name())
                                        .monospace()
                                        .size(12.0)
                                        .color(p.text_muted),
                                );
                                reg_edit(ui, p, state, view, RegId::Rgq);
                                ui.end_row();
                            });
                    });
                });
            });

            ui.add_space(8.0);

            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    card(ui, p, "Регистры блока МПУ", |ui| {
                        egui::Grid::new("mpu_grid")
                            .num_columns(4)
                            .spacing([6.0, 4.0])
                            .show(ui, |ui| {
                                labeled_edit(ui, p, state, view, RegId::Cmk);
                                labeled_edit(ui, p, state, view, RegId::Error);
                                ui.end_row();
                                labeled_edit(ui, p, state, view, RegId::Muad);
                                labeled_edit(ui, p, state, view, RegId::Ract);
                                ui.end_row();
                                labeled_edit(ui, p, state, view, RegId::Stp);
                                ui.label("");
                                ui.end_row();
                            });
                        muted_label(ui, p, "STP — глубина стека адресов возврата");
                    });
                });
                cols[1].vertical(|ui| {
                    card(ui, p, "Регистры ОП", |ui| {
                        egui::Grid::new("mem_grid")
                            .num_columns(2)
                            .spacing([6.0, 4.0])
                            .show(ui, |ui| {
                                for reg in [RegId::Aram, RegId::Rgr, RegId::Rgw] {
                                    labeled_edit(ui, p, state, view, reg);
                                    ui.end_row();
                                }
                            });
                    });
                });
            });
        });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button("Загрузить…").clicked() {
            action = Some(Action::Load);
        }
        if ui.button("Сохранить…").clicked() {
            action = Some(Action::Save);
        }
        if ui.button("Сброс").clicked() {
            action = Some(Action::Reset);
        }
    });

    action
}

fn card(ui: &mut egui::Ui, p: &Palette, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(p.card_bg)
        .corner_radius(8)
        .stroke(egui::Stroke::new(1.0, p.border))
        .inner_margin(egui::Margin::same(10))
        .outer_margin(egui::Margin { right: 4, bottom: 4, ..Default::default() })
        .show(ui, |ui| {
            section_title(ui, p, title);
            ui.add_space(2.0);
            content(ui);
        });
}

fn labeled_edit(
    ui: &mut egui::Ui,
    p: &Palette,
    state: &mut CpuState,
    view: &mut RegistersViewState,
    reg: RegId,
) {
    ui.label(
        RichText::new(reg.name())
            .monospace()
            .size(12.0)
            .color(p.text_muted),
    );
    reg_edit(ui, p, state, view, reg);
}

fn reg_edit(
    ui: &mut egui::Ui,
    p: &Palette,
    state: &mut CpuState,
    view: &mut RegistersViewState,
    reg: RegId,
) {
    let idx = reg.index();
    let changed = state.registers.is_changed(reg);
    let response = view.edits[idx].show(ui, state.registers.get(reg), 4, |v| {
        state.registers.set(reg, v);
    });
    if changed {
        ui.painter().rect_stroke(
            response.rect,
            4.0,
            egui::Stroke::new(1.0, p.success),
            egui::StrokeKind::Inside,
        );
    }
}
