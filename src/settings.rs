//! Настройки, сохраняемые между запусками: язык, уровень, цвет игрока, звук,
//! видимость панели ходов и спецэффекты.
//!
//! Файл — простой текст вида `ключ=значение`. Неизвестные ключи и испорченные
//! значения игнорируются: вместо них берутся значения по умолчанию.

use crate::ai::{clamp_level, MAX_LEVEL, MIN_LEVEL};
use crate::board::Color;
use crate::i18n::Lang;
use crate::ui::fx::EffectSet;
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
    /// Общий выключатель спецэффектов: при `false` не показывается ни один.
    pub effects_enabled: bool,
    /// Какие спецэффекты выбраны по отдельности.
    pub effects: EffectSet,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lang: Lang::DEFAULT,
            level: DEFAULT_LEVEL,
            human: Color::White,
            sound: true,
            panel: true,
            effects_enabled: true,
            effects: EffectSet::ALL,
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

const fn switch(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

impl Settings {
    /// Настройки по умолчанию с языком, выбранным по локали системы.
    #[must_use]
    pub fn for_locale(locale: Option<&str>) -> Self {
        Self {
            lang: Lang::from_locale(locale),
            ..Self::default()
        }
    }

    /// Спецэффекты, которые нужно показывать: выбранные по отдельности, если включён
    /// общий выключатель, иначе никакие.
    #[must_use]
    pub const fn active_effects(&self) -> EffectSet {
        if self.effects_enabled {
            self.effects
        } else {
            EffectSet::NONE
        }
    }

    /// Текст файла настроек.
    #[must_use]
    pub fn to_text(&self) -> String {
        format!(
            "# Russian Draughts settings\nlanguage={}\nlevel={}\ncolor={}\nsound={}\npanel={}\n\
             effects={}\neffect_lightning={}\neffect_sparks={}\neffect_flashes={}\n",
            self.lang.code(),
            clamp_level(self.level),
            match self.human {
                Color::White => "white",
                Color::Black => "black",
            },
            switch(self.sound),
            switch(self.panel),
            switch(self.effects_enabled),
            switch(self.effects.lightning),
            switch(self.effects.sparks),
            switch(self.effects.flashes),
        )
    }

    /// Разбор текста файла настроек поверх значений `base`.
    #[must_use]
    pub fn from_text(text: &str, base: Self) -> Self {
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
                "effects" => {
                    if let Some(on) = parse_switch(value) {
                        settings.effects_enabled = on;
                    }
                }
                "effect_lightning" => {
                    if let Some(on) = parse_switch(value) {
                        settings.effects.lightning = on;
                    }
                }
                "effect_sparks" => {
                    if let Some(on) = parse_switch(value) {
                        settings.effects.sparks = on;
                    }
                }
                "effect_flashes" => {
                    if let Some(on) = parse_switch(value) {
                        settings.effects.flashes = on;
                    }
                }
                _ => {}
            }
        }
        settings
    }

    /// Читает настройки из файла. Если файла нет или он не читается, возвращает `base`.
    #[must_use]
    pub fn load(path: &Path, base: Self) -> Self {
        match fs::read(path) {
            Ok(bytes) => Self::from_text(&String::from_utf8_lossy(&bytes), base),
            Err(_) => base,
        }
    }

    /// Writes the settings to the file, creating its directory if needed.
    ///
    /// The text goes to a temporary file next to the target, which then replaces the
    /// target in one step: if the program or the computer stops in the middle of
    /// saving, the previous settings survive instead of a half-written file.
    ///
    /// # Errors
    ///
    /// Returns the I/O error if the directory cannot be created or the file cannot be
    /// written or put in place.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut temporary = path.as_os_str().to_owned();
        temporary.push(".tmp");
        let temporary = PathBuf::from(temporary);
        fs::write(&temporary, self.to_text())?;
        fs::rename(&temporary, path).inspect_err(|_| {
            // Nothing more can be done if even the clean-up fails.
            let _ = fs::remove_file(&temporary);
        })
    }
}

/// Путь к файлу настроек по значениям переменных окружения.
///
/// * macOS: `~/Library/Application Support/rucheckers/settings.conf`;
/// * прочие системы: `$XDG_CONFIG_HOME/rucheckers/settings.conf`
///   или `~/.config/rucheckers/settings.conf`.
#[must_use]
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
#[must_use]
pub fn default_settings_path() -> Option<PathBuf> {
    let home = env::var("HOME").ok();
    let xdg = env::var("XDG_CONFIG_HOME").ok();
    settings_path_from(home.as_deref(), xdg.as_deref(), cfg!(target_os = "macos"))
}

/// Локаль сообщений из переменных окружения.
#[must_use]
pub fn system_locale() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| env::var(name).ok())
        .find(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory for the files of one test, removed when the test ends.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(test: &str) -> Self {
            let dir = env::temp_dir().join(format!("rucheckers-{test}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Every combination of the three effect switches.
    fn effect_sets() -> impl Iterator<Item = EffectSet> {
        (0..8u8).map(|bits| EffectSet {
            lightning: bits & 1 != 0,
            sparks: bits & 2 != 0,
            flashes: bits & 4 != 0,
        })
    }

    #[test]
    fn every_combination_of_settings_survives_writing_and_reading() {
        let mut checked = 0;
        for lang in Lang::ALL {
            for level in MIN_LEVEL..=MAX_LEVEL {
                for human in [Color::White, Color::Black] {
                    for switches in 0..8u8 {
                        for effects in effect_sets() {
                            let settings = Settings {
                                lang,
                                level,
                                human,
                                sound: switches & 1 != 0,
                                panel: switches & 2 != 0,
                                effects_enabled: switches & 4 != 0,
                                effects,
                            };
                            // The base differs from the saved value in every field, so
                            // a field that is not written or not read shows up.
                            let base = Settings {
                                lang: if lang == Lang::En { Lang::Ru } else { Lang::En },
                                level: if level == MIN_LEVEL { MAX_LEVEL } else { MIN_LEVEL },
                                human: human.opponent(),
                                sound: !settings.sound,
                                panel: !settings.panel,
                                effects_enabled: !settings.effects_enabled,
                                effects: EffectSet {
                                    lightning: !effects.lightning,
                                    sparks: !effects.sparks,
                                    flashes: !effects.flashes,
                                },
                            };
                            assert_eq!(Settings::from_text(&settings.to_text(), base), settings);
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(checked, 12 * 10 * 2 * 8 * 8);
    }

    #[test]
    fn each_change_is_kept_across_restarts() {
        // What the program does: load at start, save after every change.
        let dir = TempDir::new("restarts");
        let path = dir.0.join("rucheckers").join("settings.conf");
        let restart = || Settings::load(&path, Settings::default());
        assert_eq!(restart(), Settings::default(), "first start: no file yet");

        let changes: [fn(&mut Settings); 9] = [
            |s| s.level = 7,
            |s| s.human = Color::Black,
            |s| s.lang = Lang::Ja,
            |s| s.sound = false,
            |s| s.panel = false,
            |s| s.effects.sparks = false,
            |s| s.effects_enabled = false,
            |s| s.effects.lightning = false,
            |s| s.level = 10,
        ];
        let mut expected = Settings::default();
        for change in changes {
            let mut settings = restart();
            assert_eq!(settings, expected, "the previous run's settings are back");
            change(&mut settings);
            change(&mut expected);
            settings.save(&path).unwrap();
        }
        assert_eq!(
            restart(),
            Settings {
                lang: Lang::Ja,
                level: 10,
                human: Color::Black,
                sound: false,
                panel: false,
                effects_enabled: false,
                effects: EffectSet {
                    lightning: false,
                    sparks: false,
                    flashes: true,
                },
            }
        );
    }

    #[test]
    fn garbage_falls_back_to_the_base_values() {
        let base = Settings::default();
        for text in [
            "",
            "\0\0\0",
            "level=abc\nlanguage=xx\ncolor=green\nsound=maybe\npanel=",
            "effects=sometimes\neffect_lightning=\neffect_sparks=2\neffect_flashes=да",
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
    fn a_file_of_an_earlier_version_is_read() {
        // Before the special effects and the ten new languages there were five keys.
        let old = Settings::from_text(
            "# Русские шашки / Russian Draughts\nlanguage=en\nlevel=3\ncolor=black\nsound=off\npanel=on\n",
            Settings::default(),
        );
        assert_eq!(
            old,
            Settings {
                lang: Lang::En,
                level: 3,
                human: Color::Black,
                sound: false,
                panel: true,
                effects_enabled: true,
                effects: EffectSet::ALL,
            }
        );
    }

    #[test]
    fn effects_are_on_by_default_and_switch_separately_or_all_at_once() {
        let defaults = Settings::default();
        assert!(defaults.effects_enabled);
        assert_eq!(defaults.active_effects(), EffectSet::ALL);

        // Each effect is switched off on its own.
        let no_sparks = Settings::from_text("effect_sparks=off", defaults);
        assert_eq!(
            no_sparks.active_effects(),
            EffectSet {
                sparks: false,
                ..EffectSet::ALL
            }
        );
        let only_lightning = Settings::from_text("effect_sparks=off\neffect_flashes=off", defaults);
        assert_eq!(
            only_lightning.active_effects(),
            EffectSet {
                lightning: true,
                ..EffectSet::NONE
            }
        );

        // The main switch turns everything off, but the separate choices are kept.
        let all_off = Settings::from_text("effects=off\neffect_sparks=off", defaults);
        assert_eq!(all_off.active_effects(), EffectSet::NONE);
        assert!(!all_off.active_effects().any());
        assert_eq!(
            all_off.effects,
            EffectSet {
                sparks: false,
                ..EffectSet::ALL
            }
        );
        let back_on = Settings {
            effects_enabled: true,
            ..all_off
        };
        assert_eq!(
            back_on.active_effects(),
            EffectSet {
                sparks: false,
                ..EffectSet::ALL
            }
        );
        for key in [
            "effects=off",
            "effect_lightning=on",
            "effect_sparks=off",
            "effect_flashes=on",
        ] {
            assert!(all_off.to_text().contains(key), "{key}");
        }
    }

    #[test]
    fn the_file_is_plain_readable_text() {
        let text = Settings {
            lang: Lang::Zh,
            level: 9,
            human: Color::Black,
            ..Settings::default()
        }
        .to_text();
        assert!(text.is_ascii());
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with('#'));
        assert_eq!(
            lines[1..],
            [
                "language=zh",
                "level=9",
                "color=black",
                "sound=on",
                "panel=on",
                "effects=on",
                "effect_lightning=on",
                "effect_sparks=on",
                "effect_flashes=on",
            ]
        );
        // A level outside the range (it cannot come from the interface) is written clamped.
        let clamped = Settings {
            level: 200,
            ..Settings::default()
        };
        assert!(clamped.to_text().contains(&format!("level={MAX_LEVEL}\n")));
    }

    #[test]
    fn saving_creates_the_directory_and_replaces_the_file_in_one_step() {
        let dir = TempDir::new("save");
        let folder = dir.0.join("nested").join("deeper");
        let path = folder.join("settings.conf");
        let first = Settings {
            lang: Lang::En,
            level: 8,
            human: Color::Black,
            sound: false,
            panel: true,
            effects_enabled: true,
            effects: EffectSet {
                lightning: true,
                sparks: true,
                flashes: false,
            },
        };
        first.save(&path).unwrap();
        assert_eq!(Settings::load(&path, Settings::default()), first);

        // Saving again replaces the contents and leaves no temporary file behind.
        let second = Settings {
            lang: Lang::Ko,
            level: 2,
            ..first
        };
        second.save(&path).unwrap();
        assert_eq!(Settings::load(&path, Settings::default()), second);
        assert_eq!(fs::read_to_string(&path).unwrap(), second.to_text());
        let names: Vec<_> = fs::read_dir(&folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["settings.conf"]);
    }

    #[test]
    fn unreadable_and_unwritable_files_do_not_panic() {
        let dir = TempDir::new("failures");
        let base = Settings::default();
        assert_eq!(Settings::load(&dir.0.join("missing.conf"), base), base);
        // A directory in place of the file cannot be read and is not an error for loading.
        assert_eq!(Settings::load(&dir.0, base), base);
        // Bytes that are not UTF-8 are skipped; the readable lines still count.
        let mixed = dir.0.join("mixed.conf");
        fs::write(&mixed, b"\xff\xfe\x00garbage\nlevel=6\n\xc3\x28=on\n").unwrap();
        assert_eq!(Settings::load(&mixed, base), Settings { level: 6, ..base });

        // Saving below a regular file fails and reports the error instead of panicking.
        let blocker = dir.0.join("blocker");
        fs::write(&blocker, "not a directory").unwrap();
        assert!(base.save(&blocker.join("settings.conf")).is_err());
        // Saving over a directory fails too, and the temporary file is cleaned up.
        let occupied = dir.0.join("occupied");
        fs::create_dir(&occupied).unwrap();
        assert!(base.save(&occupied).is_err());
        assert!(!dir.0.join("occupied.tmp").exists());
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
            "a relative XDG_CONFIG_HOME is ignored"
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
    fn locale_selects_the_language_of_the_first_start() {
        assert_eq!(Settings::for_locale(Some("en_GB.UTF-8")).lang, Lang::En);
        assert_eq!(Settings::for_locale(Some("ru_RU.UTF-8")).lang, Lang::Ru);
        assert_eq!(Settings::for_locale(Some("ko_KR.UTF-8")).lang, Lang::Ko);
        assert_eq!(Settings::for_locale(None).lang, Lang::DEFAULT);
        // A saved language wins over the locale.
        let saved = Settings::from_text("language=de", Settings::for_locale(Some("fr_FR.UTF-8")));
        assert_eq!(saved.lang, Lang::De);
        let everything_else = Settings {
            lang: Lang::DEFAULT,
            ..Settings::for_locale(Some("tr_TR"))
        };
        assert_eq!(everything_else, Settings::default());
    }
}
