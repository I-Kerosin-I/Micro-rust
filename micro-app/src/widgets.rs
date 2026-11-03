//! Переиспользуемые UI-компоненты: HEX-поля, тосты, вспомогательные элементы.

use crate::theme::Palette;
use egui::{Color32, Context, FontId, Label, RichText, Sense, Stroke, TextEdit, Ui, Vec2};

/// HEX-поле с локальным буфером, фильтрацией ввода и фиксацией значения.
///
/// Пока поле в фокусе, буфер редактируется свободно; корректное значение
/// применяется сразу (как в оригинале с UpdateSourceTrigger), после потери
/// фокуса буфер синхронизируется с фактическим значением.
#[derive(Debug, Clone, Default)]
pub struct HexInput {
    buffer: String,
    focused: bool,
}

impl HexInput {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        value: u16,
        digits: usize,
        on_change: impl FnOnce(u16),
    ) -> egui::Response {
        if !self.focused {
            self.buffer = format!("{:0digits$X}", value, digits = digits);
        }
        let response = ui.add(
            TextEdit::singleline(&mut self.buffer)
                .font(egui::TextStyle::Monospace)
                .desired_width(digits as f32 * 9.0 + 16.0)
                .margin(egui::Margin::symmetric(4, 2)),
        );
        let was_focused = self.focused;
        self.focused = response.has_focus();

        if response.changed() {
            // Фильтрация недопустимых символов и ограничение длины
            let sanitized: String = self
                .buffer
                .chars()
                .filter(|c| c.is_ascii_hexdigit())
                .take(digits)
                .collect();
            let differs = sanitized != self.buffer;
            if let Ok(v) = u16::from_str_radix(&sanitized, 16) {
                on_change(v);
            }
            if differs {
                self.buffer = sanitized;
            }
        }

        if was_focused && !self.focused {
            // Потеря фокуса: фиксируем то, что реально применено
            self.buffer = format!("{:0digits$X}", value, digits = digits);
        }
        response
    }
}

/// Моноширинный текст.
pub fn mono(text: impl Into<String>) -> RichText {
    RichText::new(text.into()).monospace()
}
/// Тост-уведомление.
#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    pub created: std::time::Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

const TOAST_LIFETIME: f64 = 5.0;

/// Менеджер тостов: очередь сообщений и отрисовка в правом верхнем углу.
#[derive(Debug, Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    pub fn show(&mut self, kind: ToastKind, message: impl Into<String>) {
        let message = message.into();
        self.items.retain(|t| t.message != message);
        self.items.push(Toast {
            message,
            kind,
            created: std::time::Instant::now(),
        });
        if self.items.len() > 6 {
            self.items.remove(0);
        }
    }

    pub fn info(&mut self, message: impl Into<String>) {
        self.show(ToastKind::Info, message);
    }

    pub fn success(&mut self, message: impl Into<String>) {
        self.show(ToastKind::Success, message);
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.show(ToastKind::Error, message);
    }

    /// Отрисовка поверх интерфейса; возвращает true, если тосты отображаются.
    pub fn paint(&mut self, ctx: &Context, p: &Palette) -> bool {
        self.items
            .retain(|t| t.created.elapsed().as_secs_f64() < TOAST_LIFETIME);

        let screen = ctx.viewport_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::unique("toasts"),
        ));
        let mut y = screen.top() + 14.0;
        let visible = !self.items.is_empty();
        for toast in &self.items {
            let age = toast.created.elapsed().as_secs_f64();
            let alpha = ((TOAST_LIFETIME - age) / 0.4).clamp(0.0, 1.0) as f32;
            let color = match toast.kind {
                ToastKind::Info => p.accent,
                ToastKind::Success => p.success,
                ToastKind::Error => p.danger,
            };
            let galley = painter.layout(
                toast.message.clone(),
                FontId::proportional(13.0),
                p.text,
                340.0,
            );
            let size = galley.size() + Vec2::new(30.0, 18.0);
            let rect = egui::Rect::from_min_size(egui::pos2(screen.right() - 16.0 - size.x, y), size);
            painter.rect_filled(rect, 8.0, p.elevation_bg.gamma_multiply(alpha));
            painter.rect_stroke(
                rect,
                8.0,
                Stroke::new(1.0, color.gamma_multiply(alpha)),
                egui::StrokeKind::Outside,
            );
            painter.rect_filled(
                egui::Rect::from_min_max(
                    rect.left_top() + Vec2::new(4.0, 7.0),
                    rect.left_bottom() - Vec2::new(4.0, 7.0),
                ),
                2.0,
                color.gamma_multiply(alpha),
            );
            painter.galley(rect.left_top() + Vec2::new(18.0, 9.0), galley, p.text);
            y += size.y + 8.0;
        }
        if visible {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
        visible
    }
}

/// Цветной индикатор-точка (рисуется, а не символом — без «квадратиков»).
pub fn dot(ui: &mut Ui, color: Color32, radius: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(radius * 2.0 + 2.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), radius, color);
}

/// Полоса инструментов: отступ слева и вертикальное центрирование содержимого.
pub fn toolbar(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 8,
            right: 8,
            top: 7,
            bottom: 1,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| add(ui));
        });
}

/// Строка-подсказка под панелью инструментов (не перекрывает кнопки).
pub fn hint_line(ui: &mut Ui, p: &Palette, text: &str) {
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 8,
            right: 8,
            top: 0,
            bottom: 0,
        })
        .show(ui, |ui| {
            ui.set_height(18.0);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.label(RichText::new(text).size(11.5).color(p.text_dim));
            });
        });
}

/// Заголовок-разделитель внутри панели.
pub fn section_title(ui: &mut Ui, p: &Palette, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .size(11.0)
            .strong()
            .color(p.text_dim),
    );
}

/// Немая метка (для пустых состояний).
pub fn muted_label(ui: &mut Ui, p: &Palette, text: &str) {
    ui.add(Label::new(RichText::new(text).color(p.text_dim)));
}
