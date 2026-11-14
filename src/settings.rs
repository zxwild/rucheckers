//! Settings kept between runs: language, level, the player's colour, sound, visibility
//! of the move panel and special effects.
//!
//! The file is plain text of the form `key=value`. Unknown keys and damaged values are
//! ignored: the default values are used instead.

use crate::ai::{clamp_level, MAX_LEVEL, MIN_LEVEL};
use crate::board::Color;
use crate::i18n::Lang;
use crate::ui::fx::EffectSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The default level.
pub const DEFAULT_LEVEL: u8 = 4;
/// Name of the settings directory.
const APP_DIR: &str = "rucheckers";
/// Name of the settings file.
const FILE_NAME: &str = "settings.conf";

/// User settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Settings {
    pub lang: Lang,
    pub level: u8,
    pub human: Color,
    pub sound: bool,
    pub panel: bool,
    /// The main switch of the special effects: with `false` none of them is shown.
    pub effects_enabled: bool,
    /// Which special effects are chosen one by one.
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
    /// Default settings with the language chosen by the system locale.
    #[must_use]
    pub fn for_locale(locale: Option<&str>) -> Self {
        Self {
            lang: Lang::from_locale(locale),
            ..Self::default()
        }
    }

    /// The special effects to show: those chosen one by one if the main switch is on,
    /// none otherwise.
    #[must_use]
    pub const fn active_effects(&self) -> EffectSet {
        if self.effects_enabled {
            self.effects
        } else {
            EffectSet::NONE
        }
    }

    /// Text of the settings file.
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

    /// Parses the text of a settings file on top of the values of `base`.
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

    /// Reads the settings from a file. If there is no file or it cannot be read, returns `base`.
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

/// The convention an operating system has for the place of per-user settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Platform {
    /// `%APPDATA%\rucheckers\settings.conf`, that is
    /// `C:\Users\<name>\AppData\Roaming\rucheckers\settings.conf`.
    Windows,
    /// `~/Library/Application Support/rucheckers/settings.conf`.
    MacOs,
    /// `$XDG_CONFIG_HOME/rucheckers/settings.conf` or `~/.config/rucheckers/settings.conf`
    /// (Linux and other Unix systems).
    Unix,
}

impl Platform {
    /// The system the program is built for.
    pub const CURRENT: Self = if cfg!(windows) {
        Self::Windows
    } else if cfg!(target_os = "macos") {
        Self::MacOs
    } else {
        Self::Unix
    };
}

/// Path of the settings file under the convention of `platform`; `var` reads an
/// environment variable.
///
/// A variable that is unset, empty or holds a relative path does not name a directory:
/// the next candidate is tried, and without any the result is `None`.
#[must_use]
pub fn settings_path_from(platform: Platform, var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let dir = |name: &str| var(name).map(PathBuf::from).filter(|path| path.is_absolute());
    let base = match platform {
        Platform::Windows => match dir("APPDATA") {
            Some(roaming) => roaming,
            None => dir("USERPROFILE")?.join("AppData").join("Roaming"),
        },
        Platform::MacOs => dir("HOME")?.join("Library").join("Application Support"),
        Platform::Unix => match dir("XDG_CONFIG_HOME") {
            Some(config) => config,
            None => dir("HOME")?.join(".config"),
        },
    };
    Some(base.join(APP_DIR).join(FILE_NAME))
}

/// Path of the settings file on this system; `None` if the user's directory is unknown.
#[must_use]
pub fn default_settings_path() -> Option<PathBuf> {
    settings_path_from(Platform::CURRENT, |name| env::var_os(name))
}

/// The message locale the environment variables ask for; `var` reads a variable.
///
/// The variables are read the way gettext reads them. The locale is the first non-empty
/// one of `LC_ALL`, `LC_MESSAGES` and `LANG`. Unless it is `C` or `POSIX`, the list in
/// `LANGUAGE` (`de:en_GB:en`) takes priority: its first entry with a translation of the
/// interface wins. `None` means that the environment names no language.
#[must_use]
pub fn locale_from(var: impl Fn(&str) -> Option<OsString>) -> Option<String> {
    let text = |name: &str| {
        let value = var(name)?.to_string_lossy().trim().to_string();
        (!value.is_empty()).then_some(value)
    };
    let locale = ["LC_ALL", "LC_MESSAGES", "LANG"].into_iter().find_map(text)?;
    if Lang::is_neutral_locale(&locale) {
        return None;
    }
    let preferred = text("LANGUAGE").and_then(|list| {
        list.split(':')
            .map(str::trim)
            .find(|entry| Lang::of_locale(entry).is_some())
            .map(str::to_string)
    });
    Some(preferred.unwrap_or(locale))
}

/// The message locale named by the environment variables of this process.
///
/// Linux desktops always set them. On Windows and macOS they are usually unset, and the
/// executable then asks the system for the user's language.
#[must_use]
pub fn system_locale() -> Option<String> {
    locale_from(|name| env::var_os(name))
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
            "effects=sometimes\neffect_lightning=\neffect_sparks=2\neffect_flashes=はい",
            "level=0\nlevel=11\nlevel=-3\nlevel=999999999999",
            "====\n=\nlevel\n# comment",
            "レベル=5",
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

    /// An absolute path of the system the tests run on: a path that is absolute on
    /// Unix is relative on Windows and the other way round.
    fn absolute(parts: &[&str]) -> PathBuf {
        let mut path = PathBuf::from(if cfg!(windows) { "C:\\" } else { "/" });
        path.extend(parts);
        path
    }

    /// An environment made of the given variables.
    fn environment<'a>(vars: &'a [(&'a str, OsString)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.clone())
        }
    }

    /// Path of the settings file on `platform` for an environment of directories.
    fn path_on(platform: Platform, vars: &[(&str, &Path)]) -> Option<PathBuf> {
        let vars: Vec<(&str, OsString)> = vars
            .iter()
            .map(|(key, dir)| (*key, dir.as_os_str().to_owned()))
            .collect();
        settings_path_from(platform, environment(&vars))
    }

    #[test]
    fn paths_follow_platform_conventions() {
        let home = absolute(&["home", "u"]);
        let config = absolute(&["cfg"]);
        let roaming = absolute(&["Users", "u", "AppData", "Roaming"]);
        let profile = absolute(&["Users", "u"]);
        let file = Path::new("rucheckers").join("settings.conf");

        // Linux and other Unix systems: the XDG base directory convention.
        assert_eq!(
            path_on(Platform::Unix, &[("HOME", &home)]),
            Some(home.join(".config").join(&file))
        );
        assert_eq!(
            path_on(Platform::Unix, &[("HOME", &home), ("XDG_CONFIG_HOME", &config)]),
            Some(config.join(&file))
        );
        assert_eq!(
            path_on(
                Platform::Unix,
                &[("HOME", &home), ("XDG_CONFIG_HOME", Path::new("relative"))]
            ),
            Some(home.join(".config").join(&file)),
            "a relative XDG_CONFIG_HOME is ignored"
        );
        assert_eq!(
            path_on(
                Platform::Unix,
                &[("HOME", &home), ("XDG_CONFIG_HOME", Path::new(""))]
            ),
            Some(home.join(".config").join(&file)),
            "an empty XDG_CONFIG_HOME is ignored"
        );
        assert_eq!(
            path_on(Platform::Unix, &[("XDG_CONFIG_HOME", &config)]),
            Some(config.join(&file))
        );

        // macOS: the application support directory, whatever XDG_CONFIG_HOME says.
        assert_eq!(
            path_on(Platform::MacOs, &[("HOME", &home), ("XDG_CONFIG_HOME", &config)]),
            Some(home.join("Library").join("Application Support").join(&file))
        );

        // Windows: the roaming application data of the user.
        assert_eq!(
            path_on(
                Platform::Windows,
                &[("APPDATA", &roaming), ("USERPROFILE", &home)]
            ),
            Some(roaming.join(&file))
        );
        assert_eq!(
            path_on(Platform::Windows, &[("USERPROFILE", &profile)]),
            Some(roaming.join(&file)),
            "without APPDATA the directory is found from the profile"
        );
        assert_eq!(
            path_on(
                Platform::Windows,
                &[("HOME", &home), ("XDG_CONFIG_HOME", &config)]
            ),
            None,
            "the variables of Unix mean nothing on Windows"
        );

        // Without a usable directory there is no settings file.
        for platform in [Platform::Unix, Platform::MacOs, Platform::Windows] {
            assert_eq!(path_on(platform, &[]), None, "{platform:?}");
            let useless = [
                ("HOME", Path::new("")),
                ("APPDATA", Path::new("relative")),
                ("USERPROFILE", Path::new("")),
            ];
            assert_eq!(path_on(platform, &useless), None, "{platform:?}");
        }
    }

    #[test]
    fn the_path_on_this_system_matches_its_platform() {
        let expected = match Platform::CURRENT {
            Platform::Windows => cfg!(windows),
            Platform::MacOs => cfg!(target_os = "macos"),
            Platform::Unix => cfg!(all(unix, not(target_os = "macos"))),
        };
        assert!(expected, "{:?}", Platform::CURRENT);
        // The test environment may lack a home directory, but a path that is found is
        // absolute and ends the same way everywhere.
        if let Some(path) = default_settings_path() {
            assert!(path.is_absolute(), "{}", path.display());
            assert!(
                path.ends_with(Path::new("rucheckers").join("settings.conf")),
                "{}",
                path.display()
            );
        }
    }

    /// The locale an environment of text variables asks for.
    fn locale_of(vars: &[(&str, &str)]) -> Option<String> {
        let vars: Vec<(&str, OsString)> = vars
            .iter()
            .map(|(key, value)| (*key, OsString::from(value)))
            .collect();
        locale_from(environment(&vars))
    }

    #[test]
    fn the_environment_names_the_locale_the_way_gettext_reads_it() {
        // The first non-empty variable of the three wins.
        assert_eq!(
            locale_of(&[("LANG", "de_DE.UTF-8")]).as_deref(),
            Some("de_DE.UTF-8")
        );
        assert_eq!(
            locale_of(&[("LANG", "de_DE.UTF-8"), ("LC_MESSAGES", "fr_FR.UTF-8")]).as_deref(),
            Some("fr_FR.UTF-8")
        );
        assert_eq!(
            locale_of(&[
                ("LANG", "de_DE.UTF-8"),
                ("LC_MESSAGES", "fr_FR"),
                ("LC_ALL", "it_IT")
            ])
            .as_deref(),
            Some("it_IT")
        );
        assert_eq!(
            locale_of(&[("LC_ALL", ""), ("LC_MESSAGES", "  "), ("LANG", " ja_JP.UTF-8 ")]).as_deref(),
            Some("ja_JP.UTF-8"),
            "empty variables are skipped and spaces are trimmed"
        );

        // No variables, or the locale of no language: the environment names nothing.
        assert_eq!(locale_of(&[]), None);
        assert_eq!(locale_of(&[("LANG", "")]), None);
        for neutral in ["C", "POSIX", "c", "posix", "C.UTF-8", "POSIX.utf8"] {
            assert_eq!(locale_of(&[("LANG", neutral)]), None, "{neutral}");
            assert_eq!(
                locale_of(&[("LC_ALL", neutral), ("LANG", "de_DE.UTF-8")]),
                None,
                "{neutral}"
            );
            assert_eq!(
                locale_of(&[("LANG", neutral), ("LANGUAGE", "de:en")]),
                None,
                "LANGUAGE means nothing in the {neutral} locale"
            );
        }

        // LANGUAGE is a priority list: its first translated entry wins over the locale.
        assert_eq!(
            locale_of(&[("LANG", "en_US.UTF-8"), ("LANGUAGE", "de:en_GB:en")]).as_deref(),
            Some("de")
        );
        assert_eq!(
            locale_of(&[("LC_ALL", "en_US.UTF-8"), ("LANGUAGE", "pl:uk:pt_BR:en")]).as_deref(),
            Some("pt_BR"),
            "entries without a translation are skipped"
        );
        assert_eq!(
            locale_of(&[("LANG", "tr_TR.UTF-8"), ("LANGUAGE", "pl:uk")]).as_deref(),
            Some("tr_TR.UTF-8"),
            "a list without a translated entry leaves the locale in force"
        );
        assert_eq!(
            locale_of(&[("LANG", "ko_KR.UTF-8"), ("LANGUAGE", "")]).as_deref(),
            Some("ko_KR.UTF-8")
        );
        assert_eq!(
            locale_of(&[("LANG", "ko_KR.UTF-8"), ("LANGUAGE", "::C: :")]).as_deref(),
            Some("ko_KR.UTF-8")
        );
        assert_eq!(
            locale_of(&[("LANGUAGE", "de")]),
            None,
            "LANGUAGE alone does not set a locale"
        );
    }

    #[test]
    fn the_locale_of_the_environment_selects_the_language() {
        let lang = |vars: &[(&str, &str)]| Settings::for_locale(locale_of(vars).as_deref()).lang;
        assert_eq!(lang(&[("LANG", "en_US.UTF-8"), ("LANGUAGE", "ru:en")]), Lang::Ru);
        assert_eq!(lang(&[("LANG", "es_AR.UTF-8")]), Lang::Es);
        assert_eq!(lang(&[("LANG", "pl_PL.UTF-8")]), Lang::FALLBACK);
        assert_eq!(lang(&[("LANG", "C.UTF-8")]), Lang::DEFAULT);
        assert_eq!(lang(&[("LANG", "C")]), Lang::DEFAULT);
        assert_eq!(lang(&[]), Lang::DEFAULT);
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
