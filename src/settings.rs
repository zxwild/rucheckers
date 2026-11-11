//! Настройки, сохраняемые между запусками: язык, уровень, цвет игрока, звук
//! и видимость панели ходов.
//!
//! Файл — простой текст вида `ключ=значение`. Неизвестные ключи и испорченные
//! значения игнорируются: вместо них берутся значения по умолчанию.

use crate::ai::{clamp_level, MAX_LEVEL, MIN_LEVEL};
use crate::board::Color;
use crate::i18n::Lang;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Уровень по умолчанию.
pub const DEFAULT_LEVEL: u8 = 4;
/// Имя каталога настроек.
const APP_DIR: &str = "rucheckers";
/// Имя файла настроек.
const FILE_NAME: &str = "settings.conf";

/// Пользовательские настройки.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Settings {
    pub lang: Lang,
    pub level: u8,
    pub human: Color,
    pub sound: bool,
    pub panel: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lang: Lang::Ru,
            level: DEFAULT_LEVEL,
            human: Color::White,
            sound: true,
            panel: true,
        }
    }
}

fn parse_switch(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" | "1" => Some(true),
        "off" | "false" | "no" | "0" => Some(false),
        _ => None,
    }
}

fn switch(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

impl Settings {
    /// Настройки по умолчанию с языком, выбранным по локали системы.
    pub fn for_locale(locale: Option<&str>) -> Settings {
        Settings {
            lang: Lang::from_locale(locale),
            ..Settings::default()
        }
    }

    /// Текст файла настроек.
    pub fn to_text(&self) -> String {
        format!(
            "# Русские шашки / Russian Draughts\nlanguage={}\nlevel={}\ncolor={}\nsound={}\npanel={}\n",
            self.lang.code(),
            clamp_level(self.level),
            match self.human {
                Color::White => "white",
                Color::Black => "black",
            },
            switch(self.sound),
            switch(self.panel),
        )
    }

    /// Разбор текста файла настроек поверх значений `base`.
    pub fn from_text(text: &str, base: Settings) -> Settings {
        let mut settings = base;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim().to_ascii_lowercase().as_str() {
                "language" => {
                    if let Some(lang) = Lang::from_code(value) {
                        settings.lang = lang;
                    }
                }
                "level" => {
                    if let Ok(level) = value.parse::<u8>() {
                        if (MIN_LEVEL..=MAX_LEVEL).contains(&level) {
                            settings.level = level;
                        }
                    }
                }
                "color" => match value.to_ascii_lowercase().as_str() {
                    "white" => settings.human = Color::White,
                    "black" => settings.human = Color::Black,
                    _ => {}
                },
                "sound" => {
                    if let Some(on) = parse_switch(value) {
                        settings.sound = on;
                    }
                }
                "panel" => {
                    if let Some(on) = parse_switch(value) {
                        settings.panel = on;
                    }
                }
                _ => {}
            }
        }
        settings
    }

    /// Читает настройки из файла. Если файла нет или он не читается, возвращает `base`.
    pub fn load(path: &Path, base: Settings) -> Settings {
        match fs::read(path) {
            Ok(bytes) => Settings::from_text(&String::from_utf8_lossy(&bytes), base),
            Err(_) => base,
        }
    }

    /// Записывает настройки в файл, создавая каталог при необходимости.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, self.to_text())
    }
}

/// Путь к файлу настроек по значениям переменных окружения.
///
/// * macOS: `~/Library/Application Support/rucheckers/settings.conf`;
/// * прочие системы: `$XDG_CONFIG_HOME/rucheckers/settings.conf`
///   или `~/.config/rucheckers/settings.conf`.
pub fn settings_path_from(home: Option<&str>, xdg_config_home: Option<&str>, macos: bool) -> Option<PathBuf> {
    let non_empty = |value: Option<&str>| value.filter(|v| !v.trim().is_empty()).map(PathBuf::from);
    let base = if macos {
        non_empty(home)?.join("Library").join("Application Support")
    } else {
        match non_empty(xdg_config_home).filter(|p| p.is_absolute()) {
            Some(dir) => dir,
            None => non_empty(home)?.join(".config"),
        }
    };
    Some(base.join(APP_DIR).join(FILE_NAME))
}

/// Путь к файлу настроек в текущей системе; `None`, если домашний каталог неизвестен.
pub fn default_settings_path() -> Option<PathBuf> {
    let home = env::var("HOME").ok();
    let xdg = env::var("XDG_CONFIG_HOME").ok();
    settings_path_from(home.as_deref(), xdg.as_deref(), cfg!(target_os = "macos"))
}

/// Локаль сообщений из переменных окружения.
pub fn system_locale() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| env::var(name).ok())
        .find(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trip() {
        let all = [
            Settings::default(),
            Settings {
                lang: Lang::En,
                level: 10,
                human: Color::Black,
                sound: false,
                panel: false,
            },
            Settings {
                lang: Lang::Ru,
                level: 1,
                human: Color::Black,
                sound: true,
                panel: false,
            },
        ];
        for settings in all {
            assert_eq!(
                Settings::from_text(&settings.to_text(), Settings::default()),
                settings
            );
        }
    }

    #[test]
    fn garbage_falls_back_to_defaults() {
        let base = Settings::default();
        for text in [
            "",
            "\0\0\0",
            "level=abc\nlanguage=xx\ncolor=green\nsound=maybe\npanel=",
            "level=0\nlevel=11\nlevel=-3\nlevel=999999999999",
            "====\n=\nlevel\n# comment",
            "уровень=5",
        ] {
            assert_eq!(Settings::from_text(text, base), base, "{text:?}");
        }
        let partial = Settings::from_text("LEVEL = 7\n junk \nSound = OFF\n", base);
        assert_eq!(
            partial,
            Settings {
                level: 7,
                sound: false,
                ..base
            }
        );
    }

    #[test]
    fn save_and_load() {
        let dir = env::temp_dir().join(format!("rucheckers-test-{}", std::process::id()));
        let path = dir.join("nested").join("settings.conf");
        let settings = Settings {
            lang: Lang::En,
            level: 8,
            human: Color::Black,
            sound: false,
            panel: true,
        };
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path, Settings::default()), settings);
        assert_eq!(
            Settings::load(&dir.join("missing.conf"), Settings::default()),
            Settings::default()
        );
        // Каталог вместо файла не читается и не вызывает сбоя.
        assert_eq!(Settings::load(&dir, Settings::default()), Settings::default());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn paths_follow_platform_conventions() {
        assert_eq!(
            settings_path_from(Some("/home/u"), None, false),
            Some(PathBuf::from("/home/u/.config/rucheckers/settings.conf"))
        );
        assert_eq!(
            settings_path_from(Some("/home/u"), Some("/cfg"), false),
            Some(PathBuf::from("/cfg/rucheckers/settings.conf"))
        );
        assert_eq!(
            settings_path_from(Some("/home/u"), Some("relative"), false),
            Some(PathBuf::from("/home/u/.config/rucheckers/settings.conf")),
            "относительный XDG_CONFIG_HOME игнорируется"
        );
        assert_eq!(
            settings_path_from(Some("/Users/u"), Some("/cfg"), true),
            Some(PathBuf::from(
                "/Users/u/Library/Application Support/rucheckers/settings.conf"
            ))
        );
        assert_eq!(settings_path_from(None, None, false), None);
        assert_eq!(settings_path_from(Some(""), None, true), None);
    }

    #[test]
    fn locale_selects_language() {
        assert_eq!(Settings::for_locale(Some("en_GB.UTF-8")).lang, Lang::En);
        assert_eq!(Settings::for_locale(Some("ru_RU.UTF-8")).lang, Lang::Ru);
        assert_eq!(Settings::for_locale(None).lang, Lang::Ru);
    }
}
