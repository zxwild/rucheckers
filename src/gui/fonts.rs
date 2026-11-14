//! Fonts of the interface.
//!
//! The fonts built into egui cover the Latin and Cyrillic scripts. For Chinese,
//! Japanese and Korean the program carries small subsets of Noto Sans CJK that hold
//! exactly the characters of the interface texts (see `tools/subset_cjk_fonts.py` and
//! `assets/fonts/OFL.txt`); a test checks that every text can be drawn.

use eframe::egui::{Context, FontData, FontDefinitions, FontFamily};
use rucheckers::i18n::Lang;
use std::sync::Arc;

/// The egui font that draws Latin and Cyrillic text.
const TEXT_FONT: &str = "Ubuntu-Light";

/// A built-in CJK font subset: its name in the font list and the file contents.
type Subset = (&'static str, &'static [u8]);

const CHINESE: Subset = (
    "NotoSansCJKsc",
    include_bytes!("../../assets/fonts/NotoSansCJKsc-subset.otf"),
);
const JAPANESE: Subset = (
    "NotoSansCJKjp",
    include_bytes!("../../assets/fonts/NotoSansCJKjp-subset.otf"),
);
const KOREAN: Subset = (
    "NotoSansCJKkr",
    include_bytes!("../../assets/fonts/NotoSansCJKkr-subset.otf"),
);

/// The fonts for the interface in the language `lang`.
///
/// All three CJK subsets are always present, because the language list shows every
/// language under its native name. The subset of the chosen language comes first
/// among them: Chinese and Japanese share ideographs whose shapes differ between the
/// two.
pub fn definitions(lang: Lang) -> FontDefinitions {
    let order = match lang {
        Lang::Ja => [JAPANESE, CHINESE, KOREAN],
        Lang::Ko => [KOREAN, CHINESE, JAPANESE],
        _ => [CHINESE, JAPANESE, KOREAN],
    };
    let mut fonts = FontDefinitions::default();
    for (name, bytes) in order {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let names = fonts.families.entry(family).or_default();
        // The subsets go right after the text font and before the symbol fonts: one
        // of those has its own, different full-width parentheses.
        let after_text = names
            .iter()
            .position(|name| name == TEXT_FONT)
            .map_or(names.len(), |index| index + 1);
        names.splice(after_text..after_text, order.map(|(name, _)| name.to_owned()));
    }
    fonts
}

/// Makes the interface use the fonts for the language `lang`.
pub fn install(ctx: &Context, lang: Lang) {
    ctx.set_fonts(definitions(lang));
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{FontId, RawInput};
    use rucheckers::i18n::{level_label, tr, Text};

    /// Every text of the interface, in every language, with the fonts of any language.
    #[test]
    fn every_interface_text_can_be_drawn() {
        let mut texts: Vec<String> = Vec::new();
        for lang in Lang::ALL {
            texts.push(lang.native_name().to_owned());
            texts.extend(Text::ALL.iter().map(|&key| tr(lang, key).to_owned()));
            texts.extend((1..=10).map(|level| level_label(lang, level)));
        }
        // Symbols drawn on buttons and in the move list.
        texts.push("⏵ ⚙ 🔊 🔇 ⏮ ◀ ▶ ⏭ ☰ … 1. 12… a1:c3:e5 g3-h4".to_owned());

        for fonts_of in Lang::ALL {
            let ctx = Context::default();
            install(&ctx, fonts_of);
            // Fonts are loaded at the start of a frame.
            let _ = ctx.run(RawInput::default(), |_| {});
            for family in [FontFamily::Proportional, FontFamily::Monospace] {
                let font = FontId::new(15.0, family);
                ctx.fonts(|fonts| {
                    for text in &texts {
                        for ch in text.chars().filter(|ch| !ch.is_whitespace()) {
                            assert!(
                                fonts.has_glyph(&font, ch),
                                "fonts for {fonts_of:?} ({:?}) have no glyph for {ch:?} (U+{:04X}) in {text:?}",
                                font.family,
                                ch as u32,
                            );
                        }
                    }
                });
            }
        }
    }

    /// The check above means something: without the subsets the glyphs are missing.
    #[test]
    fn the_fonts_of_egui_alone_cannot_draw_cjk() {
        let ctx = Context::default();
        let _ = ctx.run(RawInput::default(), |_| {});
        let font = FontId::new(15.0, FontFamily::Proportional);
        ctx.fonts(|fonts| {
            for ch in ['棋', 'ロ', '한', '。', '「'] {
                assert!(!fonts.has_glyph(&font, ch), "{ch}");
            }
            for ch in [
                'Я', 'ё', 'ß', 'ğ', 'İ', 'ç', 'ñ', '—', '…', '«', '„', '“', '\u{a0}',
            ] {
                assert!(fonts.has_glyph(&font, ch), "{ch}");
            }
        });
    }

    #[test]
    fn the_chosen_language_comes_first_among_the_cjk_fonts() {
        let first_cjk = |lang: Lang| {
            let fonts = definitions(lang);
            fonts.families[&FontFamily::Proportional]
                .iter()
                .find(|name| name.starts_with("NotoSansCJK"))
                .cloned()
        };
        assert_eq!(first_cjk(Lang::Zh).as_deref(), Some(CHINESE.0));
        assert_eq!(first_cjk(Lang::Ja).as_deref(), Some(JAPANESE.0));
        assert_eq!(first_cjk(Lang::Ko).as_deref(), Some(KOREAN.0));
        assert_eq!(first_cjk(Lang::Ru).as_deref(), Some(CHINESE.0));
        for lang in Lang::ALL {
            let fonts = definitions(lang);
            for family in [FontFamily::Proportional, FontFamily::Monospace] {
                let names = &fonts.families[&family];
                // The subsets follow the text font and precede the symbol fonts.
                let text_font = names.iter().position(|name| name == TEXT_FONT).unwrap();
                for offset in 1..=3 {
                    assert!(names[text_font + offset].starts_with("NotoSansCJK"), "{lang:?}");
                }
                assert_eq!(
                    names.last().map(String::as_str),
                    Some("emoji-icon-font"),
                    "{lang:?}"
                );
                for subset in [CHINESE, JAPANESE, KOREAN] {
                    assert_eq!(
                        names.iter().filter(|name| *name == subset.0).count(),
                        1,
                        "{lang:?}"
                    );
                    assert!(fonts.font_data.contains_key(subset.0));
                }
            }
        }
    }
}
