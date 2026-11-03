//! Корневой объект приложения: компоновка, навигация, панель управления,
//! файловые операции, горячие клавиши.

use crate::settings::{FileKind, Settings};
use crate::theme::{self, Palette, Theme};
use crate::views::cpu::CpuViewState;
use crate::views::mpm::MpmViewState;
use crate::views::ram::RamViewState;
use crate::views::registers::RegistersViewState;
use crate::views::translation::TtaViewState;
use crate::widgets::{dot, mono, section_title, Toasts};
use egui::RichText;
use micro_core::{CpuEvent, CpuState, ExecutionState, RegId, RAM_SIZE};

/// Страницы навигации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Cpu,
    Mpm,
    Ram,
    Registers,
    Translation,
}

impl Page {
    pub const ALL: [Page; 5] = [
        Page::Cpu,
        Page::Mpm,
        Page::Ram,
        Page::Registers,
        Page::Translation,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Page::Cpu => "Работа",
            Page::Mpm => "Микропрограмма",
            Page::Ram => "Память",
            Page::Registers => "Регистры",
            Page::Translation => "Дешифратор",
        }
    }

}

/// Скорости непрерывного исполнения (МК/с).
const SPEEDS: [f64; 6] = [10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0];

pub struct MicroApp {
    pub cpu: CpuState,
    pub settings: Settings,
    pub page: Page,
    pub toasts: Toasts,

    // Вкладки
    pub cpu_view: CpuViewState,
    pub mpm_view: MpmViewState,
    pub ram_view: RamViewState,
    pub registers_view: RegistersViewState,
    pub tta_view: TtaViewState,

    // Текстуры схемы
    texture_dark: Option<egui::TextureHandle>,
    texture_light: Option<egui::TextureHandle>,
    applied_theme: Option<Theme>,

    // Исполнение
    last_frame: std::time::Instant,
    run_accumulator: f64,

    // Выход
    allow_close: bool,
    show_exit_confirm: bool,
}

impl MicroApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = Settings::load();
        let mut app = Self {
            cpu: CpuState::new(),
            settings,
            page: Page::Cpu,
            toasts: Toasts::default(),
            cpu_view: CpuViewState::default(),
            mpm_view: MpmViewState::default(),
            ram_view: RamViewState::new(),
            registers_view: RegistersViewState::default(),
            tta_view: TtaViewState::default(),
            texture_dark: None,
            texture_light: None,
            applied_theme: None,
            last_frame: std::time::Instant::now(),
            run_accumulator: 0.0,
            allow_close: false,
            show_exit_confirm: false,
        };
        theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, app.settings.theme);
        app.applied_theme = Some(app.settings.theme);
        app
    }

    fn palette(&self) -> Palette {
        self.settings.theme.palette()
    }

    // ------------------------------------------------------------------
    // Исполнение
    // ------------------------------------------------------------------

    fn toggle_run(&mut self) {
        match self.cpu.state {
            ExecutionState::Running => {
                self.cpu.state = ExecutionState::Paused;
            }
            ExecutionState::Stopped => {}
            _ => {
                self.cpu.state = ExecutionState::Running;
            }
        }
    }

    fn step_microcommand(&mut self) {
        if matches!(
            self.cpu.state,
            ExecutionState::Stopped | ExecutionState::Running
        ) {
            return;
        }
        if self.cpu.state == ExecutionState::ReadyToStart {
            self.cpu.state = ExecutionState::Paused;
        }
        let report = self.cpu.step();
        self.handle_report(report);
    }

    fn step_instruction(&mut self) {
        if matches!(
            self.cpu.state,
            ExecutionState::Stopped | ExecutionState::Running
        ) {
            return;
        }
        if self.cpu.state == ExecutionState::ReadyToStart {
            self.cpu.state = ExecutionState::Paused;
        }
        let report = self.cpu.step_instruction();
        self.handle_report(report);
    }

    fn restart(&mut self) {
        self.cpu.restart();
        self.cpu_view.executed = None;
        self.run_accumulator = 0.0;
    }

    fn handle_report(&mut self, report: micro_core::StepReport) {
        if let Some(mk) = report.executed {
            self.cpu_view.executed = Some(mk);
        }
        for event in report.events {
            self.show_event(event);
        }
    }

    fn show_event(&mut self, event: CpuEvent) {
        let msg = match event {
            CpuEvent::MemoryReadOutOfBounds { address, word } => format!(
                "Невозможно получить {} из ячейки 0x{address:X}: память ограничена адресом 0x{:X}.",
                if word { "слово" } else { "байт" },
                RAM_SIZE - 1
            ),
            CpuEvent::MemoryWriteOutOfBounds { address, word } => format!(
                "Невозможно записать {} в ячейку 0x{address:X}: память ограничена адресом 0x{:X}.",
                if word { "слово" } else { "байт" },
                RAM_SIZE - 1
            ),
            CpuEvent::DecodeFailed => "Не удалось дешифровать команду: в таблице преобразования нет указанного кода операции. Процессор остановлен.".to_string(),
            CpuEvent::StackEmpty => "Невозможно выполнить переход по адресу из стека: стек не содержит адресов возврата. Процессор остановлен.".to_string(),
            CpuEvent::InvalidShField => {
                "Микрокоманда с недопустимым полем SH — исполнена без сдвига.".to_string()
            }
            CpuEvent::InstructionLimit => {
                "Превышен предел шагов при исполнении команды — ЦП остановлен.".to_string()
            }
        };
        self.toasts.error(msg);
    }

    fn run_frame(&mut self, ctx: &egui::Context) {
        if self.cpu.state != ExecutionState::Running {
            self.last_frame = std::time::Instant::now();
            self.run_accumulator = 0.0;
            return;
        }
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;

        let speed = self.settings.run_speed.clamp(1.0, 5_000_000.0);
        let period = 1.0 / speed;
        self.run_accumulator += dt.min(0.25); // защита от гигантских скачков времени

        const MAX_STEPS_PER_FRAME: usize = 200_000;
        let mut steps = 0usize;
        while self.run_accumulator >= period && steps < MAX_STEPS_PER_FRAME {
            self.run_accumulator -= period;
            let report = self.cpu.step();
            self.handle_report(report);
            steps += 1;
            if self.cpu.state != ExecutionState::Running {
                break;
            }
        }
        if steps >= MAX_STEPS_PER_FRAME {
            self.run_accumulator = 0.0;
        }

        let repaint_in = period.min(1.0 / 60.0);
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(repaint_in.max(0.001)));
    }

    // ------------------------------------------------------------------
    // Файловые операции
    // ------------------------------------------------------------------

    fn open_dialog(&self, kind: FileKind) -> Option<std::path::PathBuf> {
        let mut dlg = rfd::FileDialog::new();
        dlg = match kind {
            FileKind::Mpm => dlg
                .add_filter("Микропрограммная память .XMEM", &["xmem"])
                .add_filter("Микропрограммная память .MEM", &["mem"]),
            FileKind::Ram => dlg.add_filter("Оперативная память", &["ram"]),
            FileKind::Rg => dlg.add_filter("Значения регистров", &["rg"]),
            FileKind::Tta => dlg.add_filter("Таблица преобразования адресов", &["tta"]),
            FileKind::Csv => dlg.add_filter("CSV (разделитель — точка с запятой)", &["csv"]),
        };
        if let Some(dir) = self.settings.last_dir(kind) {
            if dir.exists() {
                dlg = dlg.set_directory(dir);
            }
        }
        dlg.pick_file()
    }

    fn save_dialog(&self, kind: FileKind, default_name: &str) -> Option<std::path::PathBuf> {
        let mut dlg = rfd::FileDialog::new().set_file_name(default_name);
        dlg = match kind {
            FileKind::Mpm => dlg
                .add_filter("Микропрограммная память .XMEM", &["xmem"])
                .add_filter("Микропрограммная память .MEM", &["mem"]),
            FileKind::Ram => dlg.add_filter("Оперативная память", &["ram"]),
            FileKind::Rg => dlg.add_filter("Значения регистров", &["rg"]),
            FileKind::Tta => dlg.add_filter("Таблица преобразования адресов", &["tta"]),
            FileKind::Csv => dlg.add_filter("CSV (разделитель — точка с запятой)", &["csv"]),
        };
        if let Some(dir) = self.settings.last_dir(kind) {
            if dir.exists() {
                dlg = dlg.set_directory(dir);
            }
        }
        dlg.save_file()
    }

    fn load_mpm(&mut self) {
        let Some(path) = self.open_dialog(FileKind::Mpm) else {
            return;
        };
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                self.toasts.error(format!("Не удалось открыть файл: {e}"));
                return;
            }
        };
        let text = String::from_utf8_lossy(&data);
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let result = if ext == "mem" {
            micro_core::formats::parse_mem(&text)
        } else {
            micro_core::formats::parse_xmem(&text)
        };
        match result {
            Ok(commands) => {
                self.cpu.mpm.load_from(commands);
                self.cpu.mpm_modified = false;
                self.settings.set_last_dir(FileKind::Mpm, path.clone());
                let name = file_name(&path);
                self.toasts
                    .success(format!("Микропрограммная память загружена: {name}"));
            }
            Err(e) => self.toasts.error(format!("Ошибка загрузки МПП: {e}")),
        }
    }

    fn save_mpm(&mut self) -> bool {
        let Some(path) = self.save_dialog(FileKind::Mpm, "микропрограмма.xmem") else {
            return false;
        };
        let mut path = path;
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let commands: Vec<micro_core::MicroCommand> = self.cpu.mpm.iter().cloned().collect();
        let data = if ext == "mem" {
            micro_core::formats::write_mem(&commands)
        } else {
            if ext != "xmem" {
                path.set_extension("xmem");
            }
            micro_core::formats::write_xmem(&commands)
        };
        match std::fs::write(&path, data) {
            Ok(_) => {
                self.cpu.mpm_modified = false;
                self.settings.set_last_dir(FileKind::Mpm, path.clone());
                let name = file_name(&path);
                self.toasts
                    .success(format!("Микропрограммная память сохранена: {name}"));
                true
            }
            Err(e) => {
                self.toasts.error(format!(
                    "Ошибка сохранения МПП: {e}. Вероятно, файл занят другим процессом."
                ));
                false
            }
        }
    }

    fn load_ram(&mut self) {
        let Some(path) = self.open_dialog(FileKind::Ram) else {
            return;
        };
        match std::fs::read(&path) {
            Ok(data) => match micro_core::formats::parse_ram(&data) {
                Ok(bytes) => {
                    self.cpu.ram.clear();
                    self.cpu.ram.load_bytes(&bytes);
                    self.cpu.ram_modified = false;
                    self.settings.set_last_dir(FileKind::Ram, path.clone());
                    let name = file_name(&path);
                    self.toasts
                        .success(format!("Оперативная память загружена: {name}"));
                }
                Err(e) => self.toasts.error(format!("Ошибка загрузки ОЗУ: {e}")),
            },
            Err(e) => self.toasts.error(format!("Не удалось открыть файл: {e}")),
        }
    }

    fn save_ram(&mut self) -> bool {
        let Some(path) = self.save_dialog(FileKind::Ram, "память.ram") else {
            return false;
        };
        let data = micro_core::formats::write_ram(self.cpu.ram.bytes());
        match std::fs::write(&path, data) {
            Ok(_) => {
                self.cpu.ram_modified = false;
                self.settings.set_last_dir(FileKind::Ram, path.clone());
                self.toasts.success("Оперативная память сохранена");
                true
            }
            Err(e) => {
                self.toasts.error(format!(
                    "Ошибка сохранения ОЗУ: {e}. Вероятно, файл занят другим процессом."
                ));
                false
            }
        }
    }

    fn load_registers(&mut self) {
        let Some(path) = self.open_dialog(FileKind::Rg) else {
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => match micro_core::formats::parse_rg(&text) {
                Ok(pairs) => {
                    for (reg, value) in pairs {
                        self.cpu.registers.set_silent(reg, value);
                    }
                    // Как в оригинале: флаги из файла не загружаются
                    self.cpu.registers.set_silent(RegId::Rfi, 0);
                    self.cpu.registers.set_silent(RegId::Rfd, 0);
                    self.settings.set_last_dir(FileKind::Rg, path.clone());
                    self.toasts.success("Значения регистров загружены");
                }
                Err(e) => self.toasts.error(format!("Ошибка загрузки регистров: {e}")),
            },
            Err(e) => self.toasts.error(format!("Не удалось открыть файл: {e}")),
        }
    }

    fn save_registers(&mut self) -> bool {
        let Some(path) = self.save_dialog(FileKind::Rg, "регистры.rg") else {
            return false;
        };
        let data = micro_core::formats::write_rg(&self.cpu.registers);
        match std::fs::write(&path, data) {
            Ok(_) => {
                self.settings.set_last_dir(FileKind::Rg, path.clone());
                self.toasts.success("Значения регистров сохранены");
                true
            }
            Err(e) => {
                self.toasts.error(format!("Ошибка сохранения регистров: {e}"));
                false
            }
        }
    }

    fn load_tta(&mut self) {
        let Some(path) = self.open_dialog(FileKind::Tta) else {
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => match micro_core::formats::parse_tta(&text) {
                Ok(table) => {
                    self.cpu.table = table;
                    self.cpu.tta_modified = false;
                    self.settings.set_last_dir(FileKind::Tta, path.clone());
                    self.toasts
                        .success("Таблица преобразования адресов загружена");
                }
                Err(e) => self.toasts.error(format!(
                    "Ошибка загрузки таблицы преобразования адресов: {e}"
                )),
            },
            Err(e) => self.toasts.error(format!("Не удалось открыть файл: {e}")),
        }
    }

    fn save_tta(&mut self) -> bool {
        let Some(path) = self.save_dialog(FileKind::Tta, "таблица.tta") else {
            return false;
        };
        let data = micro_core::formats::write_tta(&self.cpu.table);
        match std::fs::write(&path, data) {
            Ok(_) => {
                self.cpu.tta_modified = false;
                self.settings.set_last_dir(FileKind::Tta, path.clone());
                self.toasts.success("Таблица преобразования адресов сохранена");
                true
            }
            Err(e) => {
                self.toasts.error(format!(
                    "Ошибка сохранения таблицы: {e}. Вероятно, файл занят другим процессом."
                ));
                false
            }
        }
    }

    fn save_trace(&mut self) {
        let Some(path) = self.save_dialog(FileKind::Csv, "трасса.csv") else {
            return;
        };
        let data = micro_core::trace::export_csv(&self.cpu.trace);
        match std::fs::write(&path, data) {
            Ok(_) => {
                self.settings.set_last_dir(FileKind::Csv, path.clone());
                self.toasts.success("Трасса сохранена");
            }
            Err(e) => self.toasts.error(format!(
                "Ошибка при сохранении файла. Вероятно, файл занят другим процессом. ({e})"
            )),
        }
    }

    fn has_unsaved(&self) -> bool {
        self.cpu.mpm_modified || self.cpu.ram_modified || self.cpu.tta_modified
    }

    /// Сохранить всё изменённое (при выходе). true — всё сохранено.
    fn save_all_modified(&mut self) -> bool {
        if self.cpu.mpm_modified && !self.save_mpm() {
            return false;
        }
        if self.cpu.ram_modified && !self.save_ram() {
            return false;
        }
        if self.cpu.tta_modified && !self.save_tta() {
            return false;
        }
        true
    }

    // ------------------------------------------------------------------
    // Панели
    // ------------------------------------------------------------------

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let p = self.palette();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("MICRO")
                    .size(17.0)
                    .strong()
                    .color(p.accent),
            );
            ui.separator();
            ui.label(
                RichText::new("Микропрограммируемый процессор")
                    .size(13.5)
                    .color(p.text_muted),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let theme_label = if self.settings.theme == Theme::Dark {
                    "Светлая тема"
                } else {
                    "Тёмная тема"
                };
                if ui
                    .button(theme_label)
                    .on_hover_text("Переключить оформление")
                    .clicked()
                {
                    self.settings.theme = self.settings.theme.toggle();
                    self.settings.save();
                }
                ui.label(
                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .size(11.5)
                        .color(p.text_dim),
                );
            });
        });
    }

    fn nav_rail(&mut self, ui: &mut egui::Ui) {
        let p = self.palette();
        ui.add_space(4.0);
        section_title(ui, &p, "Разделы");
        ui.add_space(4.0);
        for page in Page::ALL {
            let selected = self.page == page;
            let modified = match page {
                Page::Mpm => self.cpu.mpm_modified,
                Page::Ram => self.cpu.ram_modified,
                Page::Translation => self.cpu.tta_modified,
                _ => false,
            };
            let mut label = page.title().to_string();
            if modified {
                label.push_str("  •");
            }
            let color = if selected { p.accent } else { p.text };
            if ui
                .selectable_label(selected, RichText::new(label).size(14.0).color(color))
                .clicked()
            {
                self.page = page;
            }
        }

        ui.add_space(18.0);
        section_title(ui, &p, "Состояние ЦП");
        ui.add_space(4.0);
        let color = theme::state_color(&p, self.cpu.state);
        ui.horizontal(|ui| {
            dot(ui, color, 4.5);
            ui.label(
                RichText::new(theme::state_title(self.cpu.state))
                    .size(13.5)
                    .color(color),
            );
        });
        ui.label(
            RichText::new(format!(
                "CMK {:02X}   STP {:04X}",
                self.cpu.registers.get(RegId::Cmk),
                self.cpu.stp()
            ))
            .monospace()
            .size(12.0)
            .color(p.text_muted),
        );
    }

    fn transport_bar(&mut self, ui: &mut egui::Ui) {
        let p = self.palette();
        ui.horizontal(|ui| {
            let can_run = self.cpu.state != ExecutionState::Stopped;
            let can_step = !matches!(
                self.cpu.state,
                ExecutionState::Stopped | ExecutionState::Running
            );

            // Фиксированная ширина кнопки пуска — по самому широкому варианту
            // («Продолжить»), чтобы кнопки не «убегали» из-под курсора.
            let run_width = ui
                .painter()
                .layout_no_wrap(
                    "Продолжить".to_owned(),
                    egui::FontId::proportional(13.5),
                    p.text,
                )
                .size()
                .x
                + 30.0;

            if ui
                .button("Рестарт")
                .on_hover_text("Сброс ЦП (Ctrl+R)")
                .clicked()
            {
                self.restart();
            }

            let run_label = match self.cpu.state {
                ExecutionState::Running => "Стоп",
                ExecutionState::Paused => "Продолжить",
                _ => "Пуск",
            };
            let run_btn = ui
                .add_enabled(
                    can_run,
                    egui::Button::new(RichText::new(run_label).strong())
                        .min_size(egui::vec2(run_width, 26.0)),
                )
                .on_hover_text("Непрерывное исполнение (F5)");
            if run_btn.clicked() {
                self.toggle_run();
            }

            if ui
                .add_enabled(can_step, egui::Button::new("Шаг"))
                .on_hover_text("Исполнить одну микрокоманду (F8)")
                .clicked()
            {
                self.step_microcommand();
            }
            if ui
                .add_enabled(can_step, egui::Button::new("Команда"))
                .on_hover_text("Исполнить команду до выборки следующей (F9)")
                .clicked()
            {
                self.step_instruction();
            }

            ui.separator();
            if ui
                .button("Сохранить трассу…")
                .on_hover_text("Экспорт трассы в CSV")
                .clicked()
            {
                self.save_trace();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let color = theme::state_color(&p, self.cpu.state);
                ui.label(mono(format!("STP {:04X}", self.cpu.stp())).size(12.5));
                ui.label(
                    mono(format!(
                        "CMK {:02X}",
                        self.cpu.registers.get(RegId::Cmk)
                    ))
                    .size(12.5),
                );
                ui.separator();
                dot(ui, color, 4.5);
                ui.label(
                    RichText::new(theme::state_title(self.cpu.state))
                        .size(13.0)
                        .color(color),
                );
                ui.separator();
                let current = format!("{} МК/с", format_speed(self.settings.run_speed));
                egui::ComboBox::from_id_salt("speed_combo")
                    .selected_text(current)
                    .width(105.0)
                    .show_ui(ui, |ui| {
                        for speed in SPEEDS {
                            let label = format!("{} МК/с", format_speed(speed));
                            ui.selectable_value(&mut self.settings.run_speed, speed, label);
                        }
                    });
            });
        });
    }

    fn central(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        match self.page {
            Page::Cpu => {
                let dark = self
                    .texture_dark
                    .get_or_insert_with(|| load_schematic_texture(ui.ctx(), "schematic_dark", true))
                    .clone();
                let light = self
                    .texture_light
                    .get_or_insert_with(|| {
                        load_schematic_texture(ui.ctx(), "schematic_light", false)
                    })
                    .clone();
                crate::views::cpu::show(
                    ui,
                    &palette,
                    &mut self.cpu,
                    &mut self.cpu_view,
                    &dark,
                    &light,
                    self.settings.theme == Theme::Dark,
                );
            }
            Page::Mpm => {
                let action =
                    crate::views::mpm::show(ui, &palette, &mut self.cpu, &mut self.mpm_view);
                match action {
                    Some(crate::views::mpm::Action::Load) => self.load_mpm(),
                    Some(crate::views::mpm::Action::Save) => {
                        self.save_mpm();
                    }
                    None => {}
                }
            }
            Page::Ram => {
                let action =
                    crate::views::ram::show(ui, &palette, &mut self.cpu, &mut self.ram_view);
                match action {
                    Some(crate::views::ram::Action::Load) => self.load_ram(),
                    Some(crate::views::ram::Action::Save) => {
                        self.save_ram();
                    }
                    None => {}
                }
            }
            Page::Registers => {
                let action = crate::views::registers::show(
                    ui,
                    &palette,
                    &mut self.cpu,
                    &mut self.registers_view,
                );
                match action {
                    Some(crate::views::registers::Action::Load) => self.load_registers(),
                    Some(crate::views::registers::Action::Save) => {
                        self.save_registers();
                    }
                    Some(crate::views::registers::Action::Reset) => {
                        for reg in micro_core::register::ALL {
                            self.cpu.registers.set_silent(reg, 0);
                        }
                        self.toasts.info("Регистры сброшены");
                    }
                    None => {}
                }
            }
            Page::Translation => {
                let action = crate::views::translation::show(
                    ui,
                    &palette,
                    &mut self.cpu,
                    &mut self.tta_view,
                );
                match action {
                    Some(crate::views::translation::Action::Load) => self.load_tta(),
                    Some(crate::views::translation::Action::Save) => {
                        self.save_tta();
                    }
                    None => {}
                }
            }
        }
    }

    fn exit_confirm_modal(&mut self, ctx: &egui::Context) {
        if !self.show_exit_confirm {
            return;
        }
        let p = self.palette();
        let mut decision: Option<bool> = None;
        egui::Window::new("Несохранённые изменения")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let mut items = Vec::new();
                if self.cpu.mpm_modified {
                    items.push("Микропрограммная память");
                }
                if self.cpu.ram_modified {
                    items.push("Оперативная память");
                }
                if self.cpu.tta_modified {
                    items.push("Таблица преобразования адресов");
                }
                ui.label(RichText::new("Имеются несохранённые изменения в:").size(14.0));
                for item in &items {
                    ui.label(
                        RichText::new(format!("  •  {item}"))
                            .size(14.0)
                            .color(p.warning),
                    );
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Сохранить перед выходом?").size(14.0).strong());
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Сохранить и выйти").clicked() {
                        decision = Some(self.save_all_modified());
                    }
                    if ui.button("Выйти без сохранения").clicked() {
                        decision = Some(true);
                    }
                    if ui.button("Отмена").clicked() {
                        decision = Some(false);
                    }
                });
            });
        match decision {
            Some(true) => {
                self.show_exit_confirm = false;
                self.allow_close = true;
                self.settings.save();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Some(false) => self.show_exit_confirm = false,
            None => {
                ctx.request_repaint();
            }
        }
    }
}

impl eframe::App for MicroApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Смена темы
        if self.applied_theme != Some(self.settings.theme) {
            theme::apply(&ctx, self.settings.theme);
            self.applied_theme = Some(self.settings.theme);
        }

        // Запрос закрытия окна: предупреждение о несохранённых изменениях
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested && !self.allow_close && self.has_unsaved() {
            if !self.show_exit_confirm {
                self.show_exit_confirm = true;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        // Горячие клавиши
        let (f5, f8, f9, ctrl_r) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::F5),
                i.key_pressed(egui::Key::F8),
                i.key_pressed(egui::Key::F9),
                i.key_pressed(egui::Key::R) && i.modifiers.ctrl,
            )
        });
        if f5 {
            self.toggle_run();
        }
        if f8 {
            self.step_microcommand();
        }
        if f9 {
            self.step_instruction();
        }
        if ctrl_r {
            self.restart();
        }

        // Цикл непрерывного исполнения
        self.run_frame(&ctx);

        // Панели: верхняя, нижняя (управление), боковая (навигация), центральная
        let p = self.palette();
        egui::Panel::top("top_bar")
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(p.panel_bg)
                    .inner_margin(egui::Margin::symmetric(12, 7)),
            )
            .show(ui, |ui| self.top_bar(ui));

        egui::Panel::bottom("transport_bar")
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(p.panel_bg)
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ui, |ui| self.transport_bar(ui));

        egui::Panel::left("nav_rail")
            .resizable(false)
            .exact_size(185.0)
            .frame(
                egui::Frame::new()
                    .fill(p.panel_bg)
                    .inner_margin(egui::Margin::same(10)),
            )
            .show(ui, |ui| self.nav_rail(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.window_bg))
            .show(ui, |ui| self.central(ui));

        // Тосты поверх всего
        self.toasts.paint(&ctx, &p);

        // Модальное окно выхода
        self.exit_confirm_modal(&ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings.save();
    }
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn format_speed(speed: f64) -> String {
    if speed >= 1_000_000.0 {
        format!("{}", (speed / 1_000_000.0) as u64)
    } else {
        format!("{}", speed as u64)
    }
}

fn load_schematic_texture(ctx: &egui::Context, name: &str, dark: bool) -> egui::TextureHandle {
    let bytes: &'static [u8] = if dark {
        include_bytes!("../assets/Interface_Dark_Theme.png")
    } else {
        include_bytes!("../assets/Interface_Light_Theme.png")
    };
    let img = image::load_from_memory(bytes).expect("схема процессора должна декодироваться");
    let size = [img.width() as usize, img.height() as usize];
    let pixels = img.to_rgba8().into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR)
}
