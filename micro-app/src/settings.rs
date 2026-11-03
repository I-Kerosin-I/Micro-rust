//! Пользовательские настройки (JSON в стандартном каталоге конфигурации ОС).

use crate::theme::Theme;
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    /// Последние каталоги для диалогов открытия/сохранения по типам файлов.
    pub last_dir_mpm: Option<PathBuf>,
    pub last_dir_ram: Option<PathBuf>,
    pub last_dir_rg: Option<PathBuf>,
    pub last_dir_tta: Option<PathBuf>,
    pub last_dir_csv: Option<PathBuf>,
    /// Скорость непрерывного исполнения, микрокоманд в секунду.
    pub run_speed: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            last_dir_mpm: None,
            last_dir_ram: None,
            last_dir_rg: None,
            last_dir_tta: None,
            last_dir_csv: None,
            run_speed: 1000.0,
        }
    }
}

impl Settings {
    fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("micro").join("settings.json"))
    }

    /// Загрузить настройки; при отсутствии/повреждении — значения по умолчанию.
    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };
        // Повреждённый файл пересоздаётся при следующем сохранении.
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, data);
        }
    }

    pub fn last_dir(&self, kind: FileKind) -> Option<PathBuf> {
        match kind {
            FileKind::Mpm => self.last_dir_mpm.clone(),
            FileKind::Ram => self.last_dir_ram.clone(),
            FileKind::Rg => self.last_dir_rg.clone(),
            FileKind::Tta => self.last_dir_tta.clone(),
            FileKind::Csv => self.last_dir_csv.clone(),
        }
    }

    pub fn set_last_dir(&mut self, kind: FileKind, dir: PathBuf) {
        let slot = match kind {
            FileKind::Mpm => &mut self.last_dir_mpm,
            FileKind::Ram => &mut self.last_dir_ram,
            FileKind::Rg => &mut self.last_dir_rg,
            FileKind::Tta => &mut self.last_dir_tta,
            FileKind::Csv => &mut self.last_dir_csv,
        };
        *slot = dir.parent().map(|p| p.to_path_buf()).or(Some(dir));
    }
}

/// Тип файла для диалогов и запоминания каталогов.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Mpm,
    Ram,
    Rg,
    Tta,
    Csv,
}

impl FileKind {
    /// (описание, расширение)
    #[allow(dead_code)]
    pub const fn filter(self) -> (&'static str, &'static str) {
        match self {
            FileKind::Mpm => ("Микропрограммная память", "xmem,mem"),
            FileKind::Ram => ("Оперативная память", "ram"),
            FileKind::Rg => ("Значения регистров", "rg"),
            FileKind::Tta => ("Таблица преобразования адресов", "tta"),
            FileKind::Csv => ("CSV (разделитель — точка с запятой)", "csv"),
        }
    }
}
