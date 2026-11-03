//! Вкладка «Микропрограммная память»: редактор 64 микрокоманд.

use crate::theme::Palette;
use egui::{Align2, Color32, CursorIcon, FontId, RichText, Sense, Stroke, TextEdit, Ui, Vec2};
use micro_core::{field_options, CpuState, Field, MPM_SIZE};

/// Ячейка редактирования: поле МК или комментарий.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Field(Field),
    Comment,
}

/// Локальное состояние редактора.
#[derive(Debug, Default)]
pub struct MpmViewState {
    /// Активная (редактируемая) ячейка.
    active: Option<(usize, Cell)>,
    buf: String,
    /// Прокрутка к адресу (после вставки/удаления).
    scroll_to: Option<usize>,
    /// Запросить фокус ввода у активной ячейки в этом кадре.
    focus_pending: bool,
}

const CELL_W: f32 = 30.0;
const CONST_W: f32 = 46.0;
const ADDR_W: f32 = 36.0;
const ROW_H: f32 = 23.0;

impl MpmViewState {
    /// Применить значение к МК.
    fn commit(&self, state: &mut CpuState, row: usize, field: Field, buf: &str) {
        if let Ok(v) = u16::from_str_radix(buf, 16) {
            let mk = state.mpm.get_mut(row);
            mk.set(field, v);
            state.mpm_modified = true;
        }
    }
}

pub fn show(ui: &mut egui::Ui, p: &Palette, state: &mut CpuState, view: &mut MpmViewState) -> Option<Action> {
    let mut action = None;

    // Отмена активной ячейки по Esc
    if view.active.is_some() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        view.active = None;
    }

    // Панель действий (сверху, чтобы всегда была видна)
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
        "Клик по ячейке — редактирование · ПКМ — справочник значений · ПКМ по адресу — вставить/удалить МК",
    );
    ui.add_space(2.0);

    egui::Frame::new()
        .fill(p.panel_bg)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            let mut table = egui_extras::TableBuilder::new(ui)
                .id_salt("mpm_table")
                .striped(true)
                .vscroll(true)
                .min_scrolled_height(0.0);
            table = table.column(egui_extras::Column::exact(ADDR_W).resizable(false).clip(false));
            for (name, field) in FIELD_COLS.iter() {
                let _ = name;
                let w = if *field == Field::Const { CONST_W } else { CELL_W };
                table = table.column(egui_extras::Column::exact(w).resizable(false).clip(false));
            }
            table = table.column(egui_extras::Column::remainder().resizable(false).clip(false));

            if let Some(scroll_to) = view.scroll_to.take() {
                table = table.scroll_to_row(scroll_to, Some(egui::Align::Center));
            }

            table.header(22.0, |mut header| {
                header.col(|ui| {
                    ui.label(RichText::new("Адр.").size(11.0).color(p.text_dim));
                });
                for (name, _) in FIELD_COLS.iter() {
                    header.col(|ui| {
                        ui.label(RichText::new(*name).size(11.0).color(p.text_dim));
                    });
                }
                header.col(|ui| {
                    ui.label(RichText::new("Комментарий").size(11.0).color(p.text_dim));
                });
            })
            .body(|mut body| {
                for row in 0..MPM_SIZE {
                    body.row(ROW_H, |mut row_ui| {
                        render_row(p, state, view, row, &mut row_ui, &mut action);
                    });
                }
            });
        });

    action
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Load,
    Save,
}

const FIELD_COLS: [(&str, Field); 17] = [
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

fn render_row(
    p: &Palette,
    state: &mut CpuState,
    view: &mut MpmViewState,
    row: usize,
    row_ui: &mut egui_extras::TableRow<'_, '_>,
    _action: &mut Option<Action>,
) {
    // Адрес + контекстное меню вставки/удаления
    row_ui.col(|ui| {
        let addr = state.mpm.get(row).address;
        let r = text_cell(ui, p, format!("{:02X}", addr), ADDR_W - 4.0, ROW_H - 3.0, p.text_dim, false, true);
        r.context_menu(|ui| {
            if ui.button("Вставить МК (сдвиг вправо)").clicked() {
                state.mpm.insert(row);
                state.mpm_modified = true;
                ui.close();
            }
            if ui.button("Удалить МК (сдвиг влево)").clicked() {
                state.mpm.remove(row);
                state.mpm_modified = true;
                ui.close();
            }
        });
    });

    // Поля микрокоманды
    for (name, field) in FIELD_COLS {
        let _ = name;
        let w = if field == Field::Const { CONST_W } else { CELL_W };
        row_ui.col(|ui| {
            let value = state.mpm.get(row).get(field);
            let text = if field == Field::Const {
                format!("{:04X}", value)
            } else {
                format!("{:01X}", value)
            };
            let is_active = view.active == Some((row, Cell::Field(field)));
            let is_current = state.registers.get(micro_core::RegId::Cmk) as usize == row;

            if is_active {
                let response = ui.add(
                    TextEdit::singleline(&mut view.buf)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(w - 8.0)
                        .min_size(egui::Vec2::new(w - 8.0, ROW_H - 5.0))
                        .margin(egui::Margin::symmetric(4, 0))
                        .hint_text(egui::RichText::new(text).monospace().color(p.text_dim)),
                );
                if view.focus_pending {
                    response.request_focus();
                    view.focus_pending = false;
                }
                if response.changed() {
                    let digits = if field == Field::Const { 4 } else { 1 };
                    let sanitized: String = view
                        .buf
                        .chars()
                        .filter(|c| c.is_ascii_hexdigit())
                        .take(digits)
                        .collect();
                    if sanitized != view.buf {
                        view.buf = sanitized;
                    }
                }
                let enter = response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if response.lost_focus() {
                    view.commit(state, row, field, &view.buf);
                    if enter {
                        // Переход к редактированию той же колонки строкой ниже
                        view.active = Some(((row + 1) % MPM_SIZE, Cell::Field(field)));
                        let next_value = state.mpm.get((row + 1) % MPM_SIZE).get(field);
                        view.buf = if field == Field::Const {
                            format!("{:04X}", next_value)
                        } else {
                            format!("{:01X}", next_value)
                        };
                    } else {
                        view.active = None;
                    }
                }
            } else {
                let highlight = is_current;
                let resp = text_cell(
                    ui,
                    p,
                    text.clone(),
                    w - 4.0,
                    ROW_H - 3.0,
                    if highlight { p.accent } else { p.text },
                    highlight,
                    false,
                );
                if resp.clicked() {
                    view.active = Some((row, Cell::Field(field)));
                    view.buf.clear(); // ввод с чистого листа; старое значение — в подсказке
                    view.focus_pending = true;
                }
                let field_options_ctx = field;
                resp.context_menu(|ui| {
                    ui.set_min_width(180.0);
                    ui.label(
                        RichText::new(format!("Поле {} — МК {:02X}", field_options_ctx.name(), row))
                            .size(11.0)
                            .color(p.text_dim),
                    );
                    ui.separator();
                    for (value, description) in field_options(field_options_ctx) {
                        let label = if field_options_ctx == Field::Const {
                            format!("{:04X}  {}", value, description)
                        } else {
                            format!("{:X}  {}", value, description)
                        };
                        if ui.button(label).clicked() {
                            let mk = state.mpm.get_mut(row);
                            mk.set(field_options_ctx, value);
                            state.mpm_modified = true;
                            ui.close();
                        }
                    }
                });
            }
        });
    }

    // Комментарий
    row_ui.col(|ui| {
        let is_active = view.active == Some((row, Cell::Comment));
        if is_active {
            let response = ui.add(
                TextEdit::singleline(&mut view.buf)
                    .desired_width(ui.available_width())
                    .min_size(egui::Vec2::new(ui.available_width(), ROW_H - 5.0))
                    .margin(egui::Margin::symmetric(4, 0)),
            );
            if view.focus_pending {
                response.request_focus();
                view.focus_pending = false;
            }
            if response.lost_focus() {
                let comment = view.buf.trim().to_string();
                state.mpm.get_mut(row).comment = comment;
                state.mpm_modified = true;
                view.active = None;
            }
        } else {
            let comment = state.mpm.get(row).comment.clone();
            let display = if comment.is_empty() { "—".to_string() } else { comment.clone() };
            let (rect, resp) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), ROW_H - 3.0),
                Sense::click(),
            );
            ui.painter().text(
                rect.left_center(),
                Align2::LEFT_CENTER,
                display,
                FontId::proportional(12.5),
                if comment.is_empty() { p.text_dim } else { p.text },
            );
            if resp.clicked() {
                view.active = Some((row, Cell::Comment));
                view.buf = comment;
                view.focus_pending = true;
            }
        }
    });
}

/// Отрисовка текстовой ячейки с фоном/рамкой; возвращает реакцию.
#[allow(clippy::too_many_arguments)]
fn text_cell(
    ui: &mut Ui,
    p: &Palette,
    text: String,
    w: f32,
    h: f32,
    color: Color32,
    highlight: bool,
    dim: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
    let painter = ui.painter();
    let bg = if highlight {
        p.accent_soft
    } else if ui.rect_contains_pointer(rect) {
        p.card_bg_hover
    } else {
        p.card_bg
    };
    painter.rect_filled(rect, 4.0, bg);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, if highlight { p.accent } else { p.border }),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(if dim { 12.0 } else { 12.5 }),
        color,
    );
    resp.on_hover_cursor(CursorIcon::PointingHand)
}
