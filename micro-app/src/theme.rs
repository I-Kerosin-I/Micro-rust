//! Дизайн-система: палитры, типографика, стили egui.

use egui::{Color32, CornerRadius, Margin, Style, Visuals};

/// Палитра приложения.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Palette {
    pub window_bg: Color32,
    pub panel_bg: Color32,
    pub card_bg: Color32,
    pub card_bg_hover: Color32,
    pub elevation_bg: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_text: Color32,
    pub accent_soft: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub highlight: Color32,
    pub selection: Color32,
    pub schematic: &'static [u8],
}

pub const DARK: Palette = Palette {
    window_bg: Color32::from_rgb(0x14, 0x16, 0x1B),
    panel_bg: Color32::from_rgb(0x1A, 0x1D, 0x23),
    card_bg: Color32::from_rgb(0x21, 0x24, 0x2C),
    card_bg_hover: Color32::from_rgb(0x28, 0x2C, 0x36),
    elevation_bg: Color32::from_rgb(0x2A, 0x2E, 0x38),
    border: Color32::from_rgb(0x2E, 0x32, 0x3D),
    border_strong: Color32::from_rgb(0x3D, 0x42, 0x4F),
    text: Color32::from_rgb(0xE8, 0xEA, 0xED),
    text_muted: Color32::from_rgb(0x9A, 0xA1, 0xAD),
    text_dim: Color32::from_rgb(0x63, 0x69, 0x75),
    accent: Color32::from_rgb(0x5B, 0x8D, 0xEF),
    accent_hover: Color32::from_rgb(0x74, 0xA1, 0xF5),
    accent_text: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    accent_soft: Color32::from_rgb(0x2B, 0x3A, 0x55),
    success: Color32::from_rgb(0x4C, 0xC3, 0x8A),
    warning: Color32::from_rgb(0xE5, 0xB4, 0x51),
    danger: Color32::from_rgb(0xE1, 0x6A, 0x5E),
    highlight: Color32::from_rgb(0x3D, 0x5A, 0x30),
    selection: Color32::from_rgb(0x2C, 0x43, 0x66),
    schematic: include_bytes!("../assets/Interface_Dark_Theme.png"),
};

pub const LIGHT: Palette = Palette {
    window_bg: Color32::from_rgb(0xF2, 0xF3, 0xF5),
    panel_bg: Color32::from_rgb(0xFA, 0xFB, 0xFC),
    card_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    card_bg_hover: Color32::from_rgb(0xF0, 0xF3, 0xF8),
    elevation_bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    border: Color32::from_rgb(0xE0, 0xE3, 0xE8),
    border_strong: Color32::from_rgb(0xC6, 0xCB, 0xD4),
    text: Color32::from_rgb(0x1E, 0x22, 0x28),
    text_muted: Color32::from_rgb(0x5B, 0x63, 0x70),
    text_dim: Color32::from_rgb(0x9A, 0xA2, 0xAE),
    accent: Color32::from_rgb(0x2F, 0x6F, 0xDE),
    accent_hover: Color32::from_rgb(0x24, 0x5B, 0xB8),
    accent_text: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    accent_soft: Color32::from_rgb(0xDF, 0xEA, 0xFC),
    success: Color32::from_rgb(0x1E, 0x8E, 0x5A),
    warning: Color32::from_rgb(0xB5, 0x7F, 0x0D),
    danger: Color32::from_rgb(0xC9, 0x3B, 0x2E),
    highlight: Color32::from_rgb(0xD9, 0xF0, 0xDC),
    selection: Color32::from_rgb(0xD3, 0xE3, 0xFA),
    schematic: include_bytes!("../assets/Interface_Light_Theme.png"),
};

/// Тема оформления.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[derive(Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}


impl Theme {
    pub fn palette(self) -> Palette {
        match self {
            Theme::Dark => DARK,
            Theme::Light => LIGHT,
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    #[allow(dead_code)]
    pub fn title(self) -> &'static str {
        match self {
            Theme::Dark => "тёмная",
            Theme::Light => "светлая",
        }
    }
}

/// Установка шрифтов: JetBrains Mono для моноширинного текста
/// (значения регистров, таблицы, HEX-данные).
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let regular: &'static [u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
    let medium: &'static [u8] = include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf");
    fonts
        .font_data
        .insert("jetbrains_mono".into(), egui::FontData::from_static(regular).into());
    fonts
        .font_data
        .insert("jetbrains_mono_medium".into(), egui::FontData::from_static(medium).into());
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        family.insert(0, "jetbrains_mono".into());
    }
    // Запасной шрифт в пропорциональном семействе — на случай отсутствующих глифов
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.push("jetbrains_mono".into());
    }
    ctx.set_fonts(fonts);
}

/// Применить палитру и общий стиль к контексту egui.
pub fn apply(ctx: &egui::Context, theme: Theme) {
    let p = theme.palette();

    let mut visuals = match theme {
        Theme::Dark => Visuals::dark(),
        Theme::Light => Visuals::light(),
    };
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.override_text_color = Some(p.text);
    visuals.selection.bg_fill = p.selection;
    visuals.selection.stroke = egui::Stroke::new(1.0, p.accent);
    visuals.hyperlink_color = p.accent;
    visuals.window_stroke = egui::Stroke::new(1.0, p.border_strong);
    visuals.window_shadow = egui::Shadow::NONE;
    visuals.popup_shadow = egui::Shadow::NONE;
    // Фоны панелей/меню/полей ввода
    visuals.panel_fill = p.panel_bg;
    visuals.extreme_bg_color = p.card_bg;
    visuals.faint_bg_color = p.window_bg;
    visuals.window_fill = p.elevation_bg;
    visuals.widgets = egui::style::Widgets {
        noninteractive: egui::style::WidgetVisuals {
            bg_fill: p.card_bg,
            weak_bg_fill: p.card_bg,
            bg_stroke: egui::Stroke::new(1.0, p.border),
            corner_radius: CornerRadius::same(6),
            fg_stroke: egui::Stroke::new(1.0, p.text_muted),
            expansion: 0.0,
        },
        inactive: egui::style::WidgetVisuals {
            bg_fill: p.card_bg,
            weak_bg_fill: p.card_bg,
            bg_stroke: egui::Stroke::new(1.0, p.border),
            corner_radius: CornerRadius::same(6),
            fg_stroke: egui::Stroke::new(1.0, p.text),
            expansion: 0.0,
        },
        hovered: egui::style::WidgetVisuals {
            bg_fill: p.card_bg_hover,
            weak_bg_fill: p.card_bg_hover,
            bg_stroke: egui::Stroke::new(1.0, p.border_strong),
            corner_radius: CornerRadius::same(6),
            fg_stroke: egui::Stroke::new(1.0, p.text),
            expansion: 0.0,
        },
        active: egui::style::WidgetVisuals {
            bg_fill: p.accent_soft,
            weak_bg_fill: p.accent_soft,
            bg_stroke: egui::Stroke::new(1.0, p.accent),
            corner_radius: CornerRadius::same(6),
            fg_stroke: egui::Stroke::new(1.0, p.text),
            expansion: 0.0,
        },
        open: egui::style::WidgetVisuals {
            bg_fill: p.card_bg_hover,
            weak_bg_fill: p.card_bg_hover,
            bg_stroke: egui::Stroke::new(1.0, p.border_strong),
            corner_radius: CornerRadius::same(6),
            fg_stroke: egui::Stroke::new(1.0, p.text),
            expansion: 0.0,
        },
    };

    let mut style = Style {
        visuals,
        ..Default::default()
    };
    // Запрет выделения текста меток (заголовки, таблицы, статусные строки)
    style.interaction.selectable_labels = false;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 5.0);
    style.spacing.menu_margin = Margin::same(6);
    style.spacing.interact_size = egui::vec2(40.0, 26.0);
    style.spacing.scroll.bar_width = 10.0;
    style.spacing.scroll.bar_inner_margin = 3.0;
    style.spacing.scroll.bar_outer_margin = 3.0;
    style.text_styles = [
        (egui::TextStyle::Body, egui::FontId::proportional(14.0)),
        (egui::TextStyle::Button, egui::FontId::proportional(13.5)),
        (egui::TextStyle::Small, egui::FontId::proportional(12.0)),
        (egui::TextStyle::Heading, egui::FontId::proportional(20.0)),
        (egui::TextStyle::Monospace, egui::FontId::monospace(13.5)),
    ]
    .into_iter()
    .collect();
    ctx.set_global_style(style);
}

/// Цвет по состоянию исполнения ЦП.
pub fn state_color(p: &Palette, state: micro_core::ExecutionState) -> Color32 {
    use micro_core::ExecutionState::*;
    match state {
        ReadyToStart => p.text_muted,
        Running => p.success,
        Paused => p.warning,
        Stopped => p.danger,
    }
}

/// Название состояния ЦП.
pub fn state_title(state: micro_core::ExecutionState) -> &'static str {
    use micro_core::ExecutionState::*;
    match state {
        ReadyToStart => "Готов",
        Running => "Выполнение",
        Paused => "Пауза",
        Stopped => "Останов",
    }
}
