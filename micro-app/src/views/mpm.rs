//! Вкладка «Микропрограммная память»: редактор 64 микрокоманд.
//!
//! Поля микрокоманды (кроме CONST) редактируются прямым вводом с клавиатуры
//! (как в ОЗУ): клик — выбор ячейки, HEX-цифра — ввод с немедленным
//! применением и переходом вниз. CONST и комментарии — текстовый ввод.

use crate::theme::Palette;
use egui::{
    Align2, Color32, CursorIcon, FontId, Rect, RichText, Sense, Stroke, StrokeKind, TextEdit, Ui,
    Vec2, pos2,
};
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

/// Ширина дорожки стрелки в гуттере, длина наконечника
/// и отступ вертикальной линии от правой кромки (у адреса).
const LANE_W: f32 = 7.0;
const GUTTER_RIGHT_PAD: f32 = 11.0;
const ARROW_LEN: f32 = 7.0;

/// Вид стрелки перехода.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JumpKind {
    /// Безусловный переход (CHA=0, либо CHA=1/3/4 при JFI.2=1).
    Unconditional,
    /// Условный переход (CHA=1/3/4, условие из CC/JFI).
    Conditional,
}

/// Стрелка передачи управления в гуттере.
#[derive(Debug, Clone)]
struct JumpArrow {
    from: usize,
    to: usize,
    kind: JumpKind,
    lane: usize,
    /// Условие перехода для подсказки (по полю CC с учётом JFI).
    condition: String,
}

/// Имя условия перехода по полю CC (JP/JZ/JS/JO/JC/JL/JLE/JBE);
/// при инверсии (JFI.0) после J добавляется N: JNP, JNZ и т.д.
fn cc_condition_name(cc: u8, inverted: bool) -> &'static str {
    match (cc, inverted) {
        (0, false) => "JP",
        (0, true) => "JNP",
        (1, false) => "JZ",
        (1, true) => "JNZ",
        (2, false) => "JS",
        (2, true) => "JNS",
        (3, false) => "JO",
        (3, true) => "JNO",
        (4, false) => "JC",
        (4, true) => "JNC",
        (5, false) => "JL",
        (5, true) => "JNL",
        (6, false) => "JLE",
        (6, true) => "JNLE",
        (7, false) => "JBE",
        (7, true) => "JNBE",
        _ => "",
    }
}

/// Расчёт стрелок: управление передаётся не через CHA=7 (CONT).
fn compute_jumps(state: &CpuState) -> Vec<JumpArrow> {
    let mut jumps: Vec<JumpArrow> = Vec::new();
    for i in 0..MPM_SIZE {
        let mk = state.mpm.get(i);
        if mk.jfi == 5 {
            continue; // останов: управления дальше нет
        }
        match mk.cha {
            0 => {
                // CMK <- 0: безусловный переход к выборке следующей команды
                if i != 0 {
                    jumps.push(JumpArrow {
                        from: i,
                        to: 0,
                        kind: JumpKind::Unconditional,
                        lane: 0,
                        condition: "Безусловный".to_string(),
                    });
                }
            }
            1 | 3 if mk.const_ < MPM_SIZE as u16 && mk.const_ as usize != i => {
                // CJS/CJP: переход на CONST по условию из поля CC
                // (безусловный при JFI.2; инверсия — JFI.0; источник признаков — JFI.1)
                let condition = if (mk.jfi & 4) != 0 {
                    "Безусловный".to_string()
                } else {
                    let mut c = cc_condition_name(mk.cc, (mk.jfi & 1) != 0).to_string();
                    if mk.cha == 1 {
                        c.push_str(" · вызов подпрограммы");
                    }
                    if (mk.jfi & 2) != 0 {
                        c.push_str(" · признаки из RFD");
                    }
                    c
                };
                let kind = if (mk.jfi & 4) == 0 {
                    JumpKind::Conditional
                } else {
                    JumpKind::Unconditional
                };
                jumps.push(JumpArrow {
                    from: i,
                    to: mk.const_ as usize,
                    kind,
                    lane: 0,
                    condition,
                });
            }
            4 if mk.const_ < MPM_SIZE as u16 && mk.const_ as usize != i => {
                // RPCT: цикл, пока RACT != 0 (поле CC не участвует)
                jumps.push(JumpArrow {
                    from: i,
                    to: mk.const_ as usize,
                    kind: JumpKind::Conditional,
                    lane: 0,
                    condition: "цикл, пока RACT ≠ 0".to_string(),
                });
            }
            _ => {}
        }
    }
    assign_lanes(&mut jumps);
    jumps
}

fn spans_overlap(a: &JumpArrow, b: &JumpArrow) -> bool {
    let (a1, a2) = (a.from.min(a.to), a.from.max(a.to));
    let (b1, b2) = (b.from.min(b.to), b.from.max(b.to));
    a1 <= b2 && b1 <= a2
}

/// Раскладка стрелок по дорожкам, чтобы линии не накладывались
/// (интервальная раскраска по перекрытию диапазонов «откуда…куда»).
fn assign_lanes(jumps: &mut [JumpArrow]) {
    let mut order: Vec<usize> = (0..jumps.len()).collect();
    order.sort_by_key(|&i| (jumps[i].from.min(jumps[i].to), jumps[i].from));
    let mut lanes: Vec<Option<usize>> = vec![None; jumps.len()];
    for &i in &order {
        let mut lane = 0usize;
        loop {
            let busy = jumps
                .iter()
                .enumerate()
                .any(|(k, o)| lanes[k] == Some(lane) && spans_overlap(o, &jumps[i]));
            if !busy || lane >= 7 {
                break;
            }
            lane += 1;
        }
        lanes[i] = Some(lane);
    }
    for (j, lane) in jumps.iter_mut().zip(lanes) {
        j.lane = lane.unwrap_or(0);
    }
}

/// Отрисовка стрелок в гуттере (стиль дизассемблеров: вертикальные линии
/// с отростками у источников и наконечниками на целевых строках).
/// Стрелка под курсором подсвечивается, остальные приглушаются.
fn draw_jump_arrows(
    ui: &egui::Ui,
    paint_rect: Rect,
    jumps: &[JumpArrow],
    rects: &[Rect],
    p: &Palette,
) {
    let hover_pos = ui.input(|i| i.pointer.hover_pos());
    let painter = ui.painter_at(paint_rect);

    struct Geom {
        lx: f32,
        sy: f32,
        ty: f32,
        right: f32,
        hovered: bool,
    }

    let mut geoms: Vec<Geom> = Vec::with_capacity(jumps.len());
    for j in jumps {
        let (Some(src), Some(dst)) = (rects.get(j.from), rects.get(j.to)) else {
            continue;
        };
        if !src.is_finite() || !dst.is_finite() {
            continue;
        }
        // Выход — из нижней части строки-источника, вход — в верхнюю часть
        // целевой строки: на строке с входящими и исходящими стрелками
        // их точки крепления разнесены на 11 px по вертикали.
        let sy = src.bottom() - 6.0;
        let ty = dst.top() + 6.0;
        let right = src.right();
        let lx = right - GUTTER_RIGHT_PAD - j.lane as f32 * LANE_W;
        // Зоны ограничены гуттером: за его правой кромкой (в полях МК)
        // стрелка не подсвечивается
        let hovered = hover_pos.is_some_and(|h| {
            let near_v = (h.x - lx).abs() <= 4.5
                && h.y >= sy.min(ty) - 2.0
                && h.y <= sy.max(ty) + 2.0;
            let near_stub = h.y >= sy - 3.5
                && h.y <= sy + 3.5
                && h.x >= lx - 1.0
                && h.x <= right;
            let near_head = h.y >= ty - 5.0
                && h.y <= ty + 5.0
                && h.x >= lx - 1.0
                && h.x <= right;
            near_v || near_stub || near_head
        });
        geoms.push(Geom {
            lx,
            sy,
            ty,
            right,
            hovered,
        });
    }

    // Все стрелки под курсором (зоны соседних дорожек могут перекрываться)
    let mut hovered: Vec<usize> = (0..geoms.len()).filter(|&i| geoms[i].hovered).collect();
    let any_hovered = !hovered.is_empty();

    for (g, j) in geoms.iter().zip(jumps.iter()) {
        let (base, dashed) = match j.kind {
            JumpKind::Unconditional => (Stroke::new(1.5, p.accent), false),
            JumpKind::Conditional => (Stroke::new(1.2, p.text_muted), true),
        };
        let stroke = if g.hovered {
            Stroke::new(2.3, p.text)
        } else if any_hovered {
            Stroke::new(base.width, base.color.gamma_multiply(0.4))
        } else {
            base
        };
        let dashed = dashed && !g.hovered;

        // Отросток от строки-источника к дорожке
        painter.line_segment([pos2(g.right - 2.0, g.sy), pos2(g.lx, g.sy)], stroke);
        // Вертикальная линия
        if dashed {
            painter.extend(egui::Shape::dashed_line(
                &[pos2(g.lx, g.sy), pos2(g.lx, g.ty)],
                stroke,
                4.0,
                3.0,
            ));
        } else {
            painter.line_segment([pos2(g.lx, g.sy), pos2(g.lx, g.ty)], stroke);
        }
        // Линия к целевой строке и наконечник
        painter.line_segment([pos2(g.lx, g.ty), pos2(g.right - ARROW_LEN, g.ty)], stroke);
        let tri = vec![
            pos2(g.right - ARROW_LEN, g.ty - 4.5),
            pos2(g.right - ARROW_LEN, g.ty + 4.5),
            pos2(g.right - 0.5, g.ty),
        ];
        let fill = if g.hovered { p.text } else { stroke.color };
        painter.add(egui::Shape::convex_polygon(tri, fill, Stroke::NONE));
    }

    // Курсор-рука и подсказка со списком всех переходов под курсором
    if !hovered.is_empty() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        if let Some(h) = hover_pos {
            hovered.sort_by_key(|&i| (jumps[i].from, jumps[i].to));
            let tip_painter = ui.painter();
            let mut galleys: Vec<std::sync::Arc<egui::Galley>> = Vec::new();
            for &i in &hovered {
                let j = &jumps[i];
                let text = format!("МК {:02X} → МК {:02X} · {}", j.from, j.to, j.condition);
                galleys.push(tip_painter.layout_no_wrap(
                    text,
                    FontId::proportional(12.5),
                    p.text,
                ));
            }
            let width = galleys
                .iter()
                .map(|g| g.size().x)
                .fold(0.0f32, f32::max)
                + 16.0;
            let height: f32 =
                galleys.iter().map(|g| g.size().y).sum::<f32>() + 10.0 + (galleys.len() - 1) as f32 * 3.0;
            let frame = ui.clip_rect();
            let pos = pos2(
                (h.x + 14.0)
                    .min(frame.right() - width - 4.0)
                    .max(frame.left() + 2.0),
                (h.y + 12.0)
                    .min(frame.bottom() - height - 4.0)
                    .max(frame.top() + 2.0),
            );
            let rect = Rect::from_min_size(pos, Vec2::new(width, height));
            tip_painter.rect_filled(rect, 6.0, p.elevation_bg);
            tip_painter.rect_stroke(
                rect,
                6.0,
                Stroke::new(1.0, p.border_strong),
                StrokeKind::Outside,
            );
            let mut y = rect.top() + 5.0;
            for galley in &galleys {
                tip_painter.galley(pos2(rect.left() + 8.0, y), galley.clone(), p.text);
                y += galley.size().y + 3.0;
            }
        }
    }
}

/// Контекст гуттера: захват прямоугольников строк и области прокрутки.
struct Gutter<'a> {
    width: f32,
    rects: &'a mut [Rect],
    viewport_clip: &'a mut Option<Rect>,
}

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

    // Стрелки переходов (как в дизассемблерах): управление передаётся не через CHA=7
    let jumps = compute_jumps(state);
    let max_lane = jumps.iter().map(|j| j.lane).max().unwrap_or(0) as f32;
    let gutter_w = if jumps.is_empty() {
        12.0
    } else {
        (max_lane + 1.0) * LANE_W + GUTTER_RIGHT_PAD + 2.0
    };

    let mut row_rects: Vec<Rect> = vec![Rect::NOTHING; MPM_SIZE];
    let mut viewport_clip: Option<Rect> = None;

    egui::Frame::new()
        .fill(p.panel_bg)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            let mut gutter = Gutter {
                width: gutter_w,
                rects: &mut row_rects,
                viewport_clip: &mut viewport_clip,
            };
            let mut table = egui_extras::TableBuilder::new(ui)
                .id_salt("mpm_table")
                .striped(true)
                .vscroll(true)
                .min_scrolled_height(0.0);
            table = table.column(egui_extras::Column::exact(ADDR_W).resizable(false).clip(false));
            table = table.column(
                egui_extras::Column::exact(gutter.width)
                    .resizable(false)
                    .clip(false),
            );
            for (name, field) in FIELD_COLS.iter() {
                let _ = name;
                let w = if *field == Field::Const { CONST_W } else { CELL_W };
                table = table.column(egui_extras::Column::exact(w).resizable(false).clip(false));
            }
            table = table.column(
                egui_extras::Column::remainder()
                    .resizable(false)
                    .clip(true)
                    .at_least(60.0),
            );

            if let Some(scroll_to) = view.scroll_to.take() {
                table = table.scroll_to_row(scroll_to, Some(egui::Align::Center));
            }

            table.header(22.0, |mut header| {
                header.col(|ui| {
                    ui.label(RichText::new("Адр.").size(11.0).color(p.text_dim));
                });
                header.col(|ui| {
                    ui.label(RichText::new("→").size(11.0).color(p.text_dim));
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
                        render_row(p, state, view, &mut gutter, row, &mut row_ui);
                    });
                }
            });

            // Стрелки поверх таблицы (координаты строк захвачены при раскладке)
            let mut min_y = f32::INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            let mut any = false;
            for j in &jumps {
                for r in [j.from, j.to] {
                    let rect = row_rects[r];
                    if rect.is_finite() {
                        min_y = min_y.min(rect.top());
                        max_y = max_y.max(rect.bottom());
                        any = true;
                    }
                }
            }
            if any {
                let span_rect = Rect::from_min_max(
                    pos2(row_rects[0].left() - 2.0, min_y - 2.0),
                    pos2(row_rects[0].right() + 2.0, max_y + 2.0),
                );
                let paint_rect = match viewport_clip {
                    Some(clip) => span_rect.intersect(clip),
                    None => span_rect,
                };
                if paint_rect.is_positive() {
                    draw_jump_arrows(ui, paint_rect, &jumps, &row_rects, p);
                }
            }
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
    gutter: &mut Gutter<'_>,
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

    // Гуттер стрелок переходов
    row_ui.col(|ui| {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(gutter.width, ROW_H - 3.0), Sense::hover());
        gutter.rects[row] = rect;
        if gutter.viewport_clip.is_none() {
            *gutter.viewport_clip = Some(ui.clip_rect());
        }
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
