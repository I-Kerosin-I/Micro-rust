//! Вкладка «Микропрограммная память»: редактор 64 микрокоманд.
//!
//! Поля микрокоманды (кроме CONST) редактируются прямым вводом с клавиатуры
//! (как в ОЗУ): клик — выбор ячейки, HEX-цифра — ввод с немедленным
//! применением и переходом вниз. CONST и комментарии — текстовый ввод.

use crate::theme::Palette;
use egui::{Align2, Color32, CursorIcon, FontId, RichText, Sense, Stroke, TextEdit, Ui, Vec2};
use micro_core::{field_options, CpuState, Field, MPM_SIZE};

/// Ячейка прямого HEX-ввода: поле МК (CONST — отдельный текстовый режим).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedCell {
    pub row: usize,
    pub field: Field,
}

/// Локальное состояние редактора.
#[derive(Debug, Default)]
pub struct MpmViewState {
    /// Выбранная ячейка прямого HEX-ввода.
    selected: Option<SelectedCell>,
    /// Буфер ввода выбранной ячейки (1 цифра).
    buf: String,
    /// CONST в режиме текстового редактирования (строка).
    active_const: Option<usize>,
    /// Комментарий в режиме текстового редактирования (строка).
    active_comment: Option<usize>,
    /// Буфер редактора CONST.
    const_buf: String,
    /// Буфер редактора комментария.
    comment_buf: String,
    /// Прокрутка к адресу.
    scroll_to: Option<usize>,
    /// Запросить фокус у активного текстового поля.
    focus_pending: bool,
}

/// Поля прямого ввода (в порядке колонок, без CONST).
const DIRECT_FIELDS: [Field; 16] = [
    Field::A,
    Field::B,
    Field::Ma,
    Field::Mb,
    Field::Mem,
    Field::Src,
    Field::Sh,
    Field::N,
    Field::Alu,
    Field::Ccx,
    Field::F,
    Field::Dst,
    Field::Wm,
    Field::Jfi,
    Field::Cc,
    Field::Cha,
];

fn next_field(field: Field) -> Option<Field> {
    let idx = DIRECT_FIELDS.iter().position(|f| *f == field)?;
    DIRECT_FIELDS.get(idx + 1).copied()
}

fn prev_field(field: Field) -> Option<Field> {
    let idx = DIRECT_FIELDS.iter().position(|f| *f == field)?;
    if idx == 0 {
        None
    } else {
        DIRECT_FIELDS.get(idx - 1).copied()
    }
}

const CELL_W: f32 = 30.0;
const CONST_W: f32 = 46.0;
const ADDR_W: f32 = 36.0;
const ROW_H: f32 = 23.0;

/// Колонки таблицы: имя и поле.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Load,
    Save,
}

pub fn show(
    ui: &mut Ui,
    p: &Palette,
    state: &mut CpuState,
    view: &mut MpmViewState,
) -> Option<Action> {
    let mut action = None;

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
        "Клик по ячейке — выбор, ввод HEX с клавиатуры (в любой раскладке), Enter/стрелки — переход · ПКМ — справочник · ПКМ по адресу — вставить/удалить МК",
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
                        render_row(p, state, view, row, &mut row_ui);
                    });
                }
            });
        });

    // Прямой клавиатурный ввод в выбранную ячейку
    handle_keyboard(ui, state, view);

    action
}

/// Ячейка прямого ввода: отображает буфер (или подсказку) и применяет цифры.
#[allow(clippy::too_many_arguments)]
fn hex_cell(
    ui: &mut Ui,
    p: &Palette,
    state: &mut CpuState,
    view: &mut MpmViewState,
    row: usize,
    field: Field,
    w: f32,
    is_current_mk: bool,
) {
    let value = state.mpm.get(row).get(field);
    let text = format!("{:01X}", value);
    let is_selected = view.selected == Some(SelectedCell { row, field });

    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, ROW_H - 3.0), Sense::click());
    let painter = ui.painter();
    let bg = if is_selected {
        p.selection
    } else if is_current_mk {
        p.accent_soft
    } else if ui.rect_contains_pointer(rect) {
        p.card_bg_hover
    } else {
        p.card_bg
    };
    painter.rect_filled(rect, 4.0, bg);
    let border = if is_selected || is_current_mk {
        p.accent
    } else {
        p.border
    };
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, border), egui::StrokeKind::Inside);
    // Буфер ввода (если цифра уже нажата) или текущее значение как подсказка
    let (display, color) = if is_selected && !view.buf.is_empty() {
        (view.buf.clone(), p.text)
    } else if is_selected {
        (text.clone(), p.text_dim)
    } else if is_current_mk {
        (text.clone(), p.accent)
    } else {
        (text.clone(), p.text)
    };
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        display,
        FontId::monospace(12.5),
        color,
    );

    if resp.clicked() {
        view.selected = Some(SelectedCell { row, field });
        view.buf.clear();
        view.active_const = None;
        view.active_comment = None;
    }
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);

    // Справочник значений (ПКМ)
    resp.context_menu(|ui| {
        ui.set_min_width(180.0);
        ui.label(
            RichText::new(format!("Поле {} — МК {:02X}", field.name(), row))
                .size(11.0)
                .color(p.text_dim),
        );
        ui.separator();
        for (value, description) in field_options(field) {
            if ui.button(format!("{:X}  {}", value, description)).clicked() {
                state.mpm.get_mut(row).set(field, value);
                state.mpm_modified = true;
                ui.close();
            }
        }
    });
}

/// Ячейка CONST: текстовый редактор (4 HEX-цифры).
fn const_cell(
    ui: &mut Ui,
    p: &Palette,
    state: &mut CpuState,
    view: &mut MpmViewState,
    row: usize,
    w: f32,
    is_current_mk: bool,
) {
    let value = state.mpm.get(row).const_;
    let is_active = view.active_const == Some(row);

    if is_active {
        let response = ui.add(
            TextEdit::singleline(&mut view.const_buf)
                .font(egui::TextStyle::Monospace)
                .desired_width(w - 8.0)
                .min_size(egui::Vec2::new(w - 8.0, ROW_H - 5.0))
                .margin(egui::Margin::symmetric(4, 0))
                .hint_text(
                    egui::RichText::new(format!("{:04X}", value))
                        .monospace()
                        .color(p.text_dim),
                ),
        );
        if view.focus_pending {
            response.request_focus();
            view.focus_pending = false;
        }
        if response.changed() {
            view.const_buf = crate::widgets::normalize_hex_string(&view.const_buf, 4);
        }
        let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if response.lost_focus() {
            if let Ok(v) = u16::from_str_radix(&view.const_buf, 16) {
                state.mpm.get_mut(row).const_ = v;
                state.mpm_modified = true;
            }
            if enter && row + 1 < MPM_SIZE {
                view.active_const = Some(row + 1);
                view.const_buf.clear();
                view.focus_pending = true;
            } else {
                view.active_const = None;
            }
        }
    } else {
        let text = format!("{:04X}", value);
        let resp = text_cell(
            ui,
            p,
            text,
            w - 4.0,
            ROW_H - 3.0,
            if is_current_mk { p.accent } else { p.text },
            is_current_mk,
            false,
        );
        if resp.clicked() {
            view.active_const = Some(row);
            view.const_buf.clear();
            view.selected = None;
            view.focus_pending = true;
        }
        resp.on_hover_cursor(CursorIcon::PointingHand);
    }
}

/// Ячейка комментария: текстовый редактор.
fn comment_cell(
    ui: &mut Ui,
    p: &Palette,
    state: &mut CpuState,
    view: &mut MpmViewState,
    row: usize,
) {
    let is_active = view.active_comment == Some(row);
    if is_active {
        let response = ui.add(
            TextEdit::singleline(&mut view.comment_buf)
                .desired_width(ui.available_width())
                .min_size(egui::Vec2::new(ui.available_width(), ROW_H - 5.0))
                .margin(egui::Margin::symmetric(4, 0)),
        );
        if view.focus_pending {
            response.request_focus();
            view.focus_pending = false;
        }
        if response.lost_focus() {
            let comment = view.comment_buf.trim().to_string();
            state.mpm.get_mut(row).comment = comment;
            state.mpm_modified = true;
            view.active_comment = None;
        }
    } else {
        let comment = state.mpm.get(row).comment.clone();
        let display = if comment.is_empty() {
            "—".to_string()
        } else {
            comment.clone()
        };
        let (rect, resp) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H - 3.0), Sense::click());
        ui.painter().text(
            rect.left_center(),
            Align2::LEFT_CENTER,
            display,
            FontId::proportional(12.5),
            if comment.is_empty() { p.text_dim } else { p.text },
        );
        if resp.clicked() {
            view.active_comment = Some(row);
            view.comment_buf = comment;
            view.selected = None;
            view.focus_pending = true;
        }
        resp.on_hover_cursor(CursorIcon::PointingHand);
    }
}

fn render_row(
    p: &Palette,
    state: &mut CpuState,
    view: &mut MpmViewState,
    row: usize,
    row_ui: &mut egui_extras::TableRow<'_, '_>,
) {
    let is_current_mk = state.registers.get(micro_core::RegId::Cmk) as usize == row;

    // Адрес + контекстное меню вставки/удаления
    row_ui.col(|ui| {
        let addr = state.mpm.get(row).address;
        let r = text_cell(
            ui,
            p,
            format!("{:02X}", addr),
            ADDR_W - 4.0,
            ROW_H - 3.0,
            p.text_dim,
            false,
            true,
        );
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
    for (name, field) in FIELD_COLS.iter() {
        let _ = name;
        let w = if *field == Field::Const { CONST_W } else { CELL_W };
        row_ui.col(|ui| {
            if *field == Field::Const {
                const_cell(ui, p, state, view, row, w, is_current_mk);
            } else {
                hex_cell(ui, p, state, view, row, *field, w - 4.0, is_current_mk);
            }
        });
    }

    // Комментарий
    row_ui.col(|ui| {
        comment_cell(ui, p, state, view, row);
    });
}

/// Прямой клавиатурный ввод в выбранную ячейку:
/// HEX-цифра — немедленное применение и переход вниз,
/// Enter/стрелки — навигация, Backspace — сброс, Esc — отмена.
fn handle_keyboard(ui: &mut Ui, state: &mut CpuState, view: &mut MpmViewState) {
    if ui.ctx().egui_wants_keyboard_input() {
        return; // фокус в текстовом поле (CONST/комментарий/поле адреса)
    }
    let Some(cell) = view.selected else {
        return;
    };
    let row = cell.row;
    let field = cell.field;

    let mut digit: Option<char> = None;
    let mut new_sel: Option<SelectedCell> = None;
    let mut deselect = false;
    ui.ctx().input(|i| {
        for event in &i.events {
            if let egui::Event::Key {
                key,
                pressed: true,
                ..
            } = event
            {
                // HEX-цифра по коду клавиши — в любой раскладке
                if let Some(d) = crate::widgets::hex_digit_from_key(*key) {
                    digit = Some(d);
                    continue;
                }
                match key {
                    egui::Key::Backspace => view.buf.clear(),
                    egui::Key::Enter => new_sel = Some(SelectedCell { row: row + 1, field }),
                    egui::Key::Escape => deselect = true,
                    egui::Key::ArrowDown => new_sel = Some(SelectedCell { row: row + 1, field }),
                    egui::Key::ArrowUp => {
                        new_sel = Some(SelectedCell {
                            row: row.saturating_sub(1),
                            field,
                        })
                    }
                    egui::Key::ArrowRight => {
                        if let Some(f) = next_field(field) {
                            new_sel = Some(SelectedCell { row, field: f });
                        }
                    }
                    egui::Key::ArrowLeft => {
                        if let Some(f) = prev_field(field) {
                            new_sel = Some(SelectedCell { row, field: f });
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    if deselect {
        view.selected = None;
        view.buf.clear();
        return;
    }

    if let Some(d) = digit {
        // Поле — одна цифра: немедленное применение и переход вниз
        view.buf.clear();
        view.buf.push(d);
        if let Ok(v) = u16::from_str_radix(&view.buf, 16) {
            state.mpm.get_mut(row).set(field, v);
            state.mpm_modified = true;
        }
        view.buf.clear();
        new_sel = Some(SelectedCell {
            row: row + 1,
            field,
        });
    }

    if let Some(next) = new_sel {
        let clamped = SelectedCell {
            row: next.row.min(MPM_SIZE - 1),
            field: next.field,
        };
        view.selected = Some(clamped);
        view.buf.clear();
    }
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
