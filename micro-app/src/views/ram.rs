//! Вкладка «Память»: HEX/ASCII-редактор ОЗУ (байты/слова).

use crate::theme::Palette;
use crate::widgets::section_title;
use egui::{Align2, Color32, CursorIcon, FontId, RichText, Sense, Stroke, Ui, Vec2};
use micro_core::{CpuState, RAM_SIZE};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RamMode {
    #[default]
    Bytes,
    Words,
}

/// Локальное состояние редактора ОЗУ.
#[derive(Debug, Default)]
pub struct RamViewState {
    pub mode: RamMode,
    pub goto_text: String,
    /// Выбранная HEX-ячейка (адрес байта).
    pub selected: Option<u16>,
    /// Выбранная ASCII-ячейка (адрес байта).
    pub ascii_selected: Option<u16>,
    /// Накапливаемые цифры при клавиатурном HEX-вводе.
    buf: String,
    /// Строка для прокрутки (адрес/16).
    pub scroll_to: Option<usize>,
}

impl RamViewState {
    pub fn new() -> Self {
        Self {
            goto_text: "000".to_string(),
            ..Default::default()
        }
    }

    fn digits(&self) -> usize {
        match self.mode {
            RamMode::Bytes => 2,
            RamMode::Words => 4,
        }
    }

    fn cell_stride(&self) -> u16 {
        match self.mode {
            RamMode::Bytes => 1,
            RamMode::Words => 2,
        }
    }

    fn cell_width(&self) -> f32 {
        match self.mode {
            RamMode::Bytes => 30.0,
            RamMode::Words => 54.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Load,
    Save,
}

const ROW_H: f32 = 21.0;

pub fn show(
    ui: &mut egui::Ui,
    p: &Palette,
    state: &mut CpuState,
    view: &mut RamViewState,
) -> Option<Action> {
    let mut action = None;

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
        "Клик по ячейке — выбор · HEX-цифры — ввод (в любой раскладке) · в колонке ASCII — ввод символа · Enter — вниз · Esc — отмена",
    );
    ui.add_space(2.0);

    egui::Frame::new()
        .fill(p.panel_bg)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            // Панель режима и перехода
            ui.horizontal(|ui| {
                section_title(ui, p, "Режим");
                let mut mode_btn = |ui: &mut Ui, label: &str, mode: RamMode| {
                    let selected = view.mode == mode;
                    if ui
                        .selectable_label(selected, RichText::new(label).size(13.0))
                        .clicked()
                    {
                        view.mode = mode;
                        view.buf.clear();
                        view.selected = None;
                        view.ascii_selected = None;
                    }
                };
                mode_btn(ui, "Байты", RamMode::Bytes);
                mode_btn(ui, "Слова", RamMode::Words);

                ui.separator();
                section_title(ui, p, "Переход");
                let goto_resp = ui.add(
                    egui::TextEdit::singleline(&mut view.goto_text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(52.0)
                        .hint_text("000"),
                );
                if goto_resp.changed() {
                    let sanitized: String = view
                        .goto_text
                        .chars()
                        .filter(|c| c.is_ascii_hexdigit())
                        .take(3)
                        .collect();
                    view.goto_text = sanitized;
                }
                let goto_address = u16::from_str_radix(&view.goto_text, 16).ok();
                let goto_valid = goto_address.is_some_and(|a| (a as usize) < RAM_SIZE);
                if ui
                    .add_enabled(goto_valid, egui::Button::new("Перейти"))
                    .on_hover_text(if goto_valid {
                        "Прокрутить таблицу к указанному адресу"
                    } else {
                        "Введите адрес в диапазоне 000…3FF (HEX)"
                    })
                    .clicked()
                {
                    if let Some(addr) = goto_address {
                        if (addr as usize) < RAM_SIZE {
                            // Прокрутка к строке адреса (однократно, в этом кадре)
                            view.scroll_to = Some(addr as usize / 16);
                        }
                    }
                }
            });

            ui.add_space(4.0);

            let mut table = egui_extras::TableBuilder::new(ui)
                .id_salt("ram_table")
                .striped(true)
                .vscroll(true)
                .min_scrolled_height(0.0);

            let cells_per_row = if view.mode == RamMode::Bytes { 16 } else { 8 };
            table = table.column(egui_extras::Column::exact(40.0).resizable(false).clip(false));
            for _ in 0..cells_per_row {
                table = table.column(
                    egui_extras::Column::exact(view.cell_width() + 4.0)
                        .resizable(false)
                        .clip(false),
                );
            }
            if view.mode == RamMode::Bytes {
                // ASCII-колонка: 16 ячеек по 14px
                table = table.column(
                    egui_extras::Column::exact(16.0 * 14.0 + 8.0)
                        .resizable(false)
                        .clip(false),
                );
            }

            // Прокрутка к строке (после «Перейти»)
            if let Some(row_index) = view.scroll_to.take() {
                table = table.scroll_to_row(row_index, Some(egui::Align::Center));
            }

            table.header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label(RichText::new("Ад").size(11.0).color(p.text_dim));
                });
                for i in 0..cells_per_row {
                    header.col(|ui| {
                        let label = if view.mode == RamMode::Bytes {
                            format!("{:X}", i)
                        } else {
                            format!("+{:X}", i * 2)
                        };
                        ui.label(RichText::new(label).size(11.0).color(p.text_dim));
                    });
                }
                if view.mode == RamMode::Bytes {
                    header.col(|ui| {
                        ui.label(RichText::new("ASCII").size(11.0).color(p.text_dim));
                    });
                }
            })
            .body(|mut body| {
                for row in 0..(RAM_SIZE / 16) {
                    body.row(ROW_H, |mut row_ui| {
                        // Адрес строки
                        row_ui.col(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(Vec2::new(36.0, 17.0), Sense::hover());
                            ui.painter().text(
                                rect.center(),
                                Align2::CENTER_CENTER,
                                format!("{:02X}", row),
                                FontId::monospace(12.0),
                                p.text_dim,
                            );
                        });
                        // HEX-ячейки
                        for c in 0..cells_per_row {
                            let addr = (row * 16 + c * view.cell_stride() as usize) as u16;
                            row_ui.col(|ui| {
                                hex_cell(ui, p, state, view, addr);
                            });
                        }
                        // ASCII-колонка (только в байтовом режиме)
                        if view.mode == RamMode::Bytes {
                            row_ui.col(|ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 0.0;
                                    for b in 0..16usize {
                                        let addr = (row * 16 + b) as u16;
                                        ascii_cell(ui, p, state, view, addr);
                                    }
                                });
                            });
                        }
                    });
                }
            });
        });

    // Клавиатурный ввод
    handle_keyboard(ui, p, state, view);

    action
}

/// Одна HEX-ячейка ОЗУ.
fn hex_cell(ui: &mut Ui, p: &Palette, state: &mut CpuState, view: &mut RamViewState, addr: u16) {
    let value = match view.mode {
        RamMode::Bytes => state.ram.get(addr) as u16,
        RamMode::Words => state.ram.get_word(addr).unwrap_or(0),
    };
    let digits = view.digits();
    let is_selected = view.selected == Some(addr);
    let is_aram = state.registers.get(micro_core::RegId::Aram) == addr;
    let changed = state.ram.is_changed(addr);

    let (rect, resp) = ui.allocate_exact_size(Vec2::new(view.cell_width(), 17.0), Sense::click());
    let painter = ui.painter();
    let bg = if is_selected {
        p.selection
    } else if changed {
        p.highlight
    } else if is_aram {
        p.accent_soft
    } else if ui.rect_contains_pointer(rect) {
        p.card_bg_hover
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        painter.rect_filled(rect, 3.0, bg);
    }
    if is_selected || is_aram {
        painter.rect_stroke(rect, 3.0, Stroke::new(1.0, p.accent), egui::StrokeKind::Inside);
    }
    let text = format!("{:0digits$X}", value, digits = digits);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(12.0),
        p.text,
    );

    if resp.clicked() {
        view.selected = if is_selected { None } else { Some(addr) };
        view.ascii_selected = None;
        view.buf.clear();
    }
    resp.on_hover_cursor(CursorIcon::PointingHand);
}

/// Одна ASCII-ячейка ОЗУ (только байтовый режим).
fn ascii_cell(ui: &mut Ui, p: &Palette, state: &mut CpuState, view: &mut RamViewState, addr: u16) {
    let v = state.ram.get(addr);
    let ch = if (0x20..0x7F).contains(&v) {
        v as char
    } else {
        '·'
    };
    let is_selected = view.ascii_selected == Some(addr);

    let (rect, resp) = ui.allocate_exact_size(Vec2::new(14.0, 17.0), Sense::click());
    let painter = ui.painter();
    if is_selected {
        painter.rect_filled(rect, 2.0, p.selection);
        painter.rect_stroke(rect, 2.0, Stroke::new(1.0, p.accent), egui::StrokeKind::Inside);
    } else if ui.rect_contains_pointer(rect) {
        painter.rect_filled(rect, 2.0, p.card_bg_hover);
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        ch.to_string(),
        FontId::monospace(11.5),
        if is_selected { p.text } else { p.text_muted },
    );

    if resp.clicked() {
        view.ascii_selected = if is_selected { None } else { Some(addr) };
        view.selected = None;
        view.buf.clear();
    }
    resp.on_hover_cursor(CursorIcon::Text);
}

/// Клавиатурный ввод: HEX-цифры (по кодам клавиш, независимо от раскладки)
/// или символы ASCII для выбранной ASCII-ячейки.
fn handle_keyboard(ui: &mut Ui, p: &Palette, state: &mut CpuState, view: &mut RamViewState) {
    if ui.ctx().egui_wants_keyboard_input() {
        return; // фокус в текстовом поле (например, адрес перехода)
    }

    // --- ASCII-ввод ---
    if let Some(addr) = view.ascii_selected {
        let mut typed: Option<u8> = None;
        let mut move_by: Option<u16> = None;
        let mut deselect = false;
        ui.ctx().input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Text(text) => {
                        if let Some(c) = text.chars().next() {
                            if (0x20..0x7F).contains(&(c as u32)) {
                                typed = Some(c as u8);
                                move_by = Some(1);
                            }
                        }
                    }
                    egui::Event::Key {
                        key,
                        pressed: true,
                        ..
                    } => match key {
                        egui::Key::Enter => move_by = Some(16),
                        egui::Key::Escape => deselect = true,
                        egui::Key::ArrowRight => move_by = Some(1),
                        egui::Key::ArrowLeft => move_by = Some(u16::MAX), // -1
                        egui::Key::ArrowDown => move_by = Some(16),
                        egui::Key::ArrowUp => move_by = Some(u16::MAX - 15),
                        _ => {}
                    },
                    _ => {}
                }
            }
        });

        if let Some(value) = typed {
            state.ram.set(addr, value);
            state.ram_modified = true;
        }
        let mut next = addr;
        if let Some(delta) = move_by {
            next = addr.wrapping_add(delta);
        }
        if deselect {
            view.ascii_selected = None;
        } else if (typed.is_some() || move_by.is_some())
            && (next as usize) < RAM_SIZE {
                view.ascii_selected = Some(next);
            }
        return;
    }

    // --- HEX-ввод (по кодам клавиш — независимо от раскладки) ---
    let Some(selected) = view.selected else {
        return;
    };

    let digits_max = view.digits();
    let mut new_selected: Option<u16> = None;
    let mut commit_value: Option<u16> = None;

    ui.ctx().input(|i| {
        for event in &i.events {
            if let egui::Event::Key {
                key,
                pressed: true,
                ..
            } = event
            {
                // Цифры и A-F по коду клавиши
                let digit = match key {
                    egui::Key::Num0 => Some('0'),
                    egui::Key::Num1 => Some('1'),
                    egui::Key::Num2 => Some('2'),
                    egui::Key::Num3 => Some('3'),
                    egui::Key::Num4 => Some('4'),
                    egui::Key::Num5 => Some('5'),
                    egui::Key::Num6 => Some('6'),
                    egui::Key::Num7 => Some('7'),
                    egui::Key::Num8 => Some('8'),
                    egui::Key::Num9 => Some('9'),
                    egui::Key::A => Some('A'),
                    egui::Key::B => Some('B'),
                    egui::Key::C => Some('C'),
                    egui::Key::D => Some('D'),
                    egui::Key::E => Some('E'),
                    egui::Key::F => Some('F'),
                    _ => None,
                };
                if let Some(c) = digit {
                    if view.buf.len() < digits_max {
                        view.buf.push(c);
                        if view.buf.len() == digits_max {
                            commit_value = u16::from_str_radix(&view.buf, 16).ok();
                            new_selected = Some(selected + view.cell_stride());
                        }
                    }
                    continue;
                }
                match key {
                    egui::Key::Backspace => {
                        view.buf.pop();
                    }
                    egui::Key::Enter => new_selected = Some(selected + 16),
                    egui::Key::Escape => view.selected = None,
                    egui::Key::ArrowRight => new_selected = Some(selected + view.cell_stride()),
                    egui::Key::ArrowLeft => {
                        new_selected = Some(selected.saturating_sub(view.cell_stride()))
                    }
                    egui::Key::ArrowDown => new_selected = Some(selected + 16),
                    egui::Key::ArrowUp => new_selected = Some(selected.saturating_sub(16)),
                    _ => {}
                }
            }
        }
    });

    if let Some(value) = commit_value {
        match view.mode {
            RamMode::Bytes => state.ram.set(selected, value as u8),
            RamMode::Words => {
                state.ram.set_word(selected, value);
            }
        }
        state.ram_modified = true;
        view.buf.clear();
    }

    if let Some(next) = new_selected {
        if (next as usize) < RAM_SIZE {
            view.selected = Some(next);
        } else {
            view.selected = Some(selected);
        }
        view.buf.clear();
    }
    let _ = p;
}
