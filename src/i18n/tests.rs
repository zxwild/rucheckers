use super::*;
use std::collections::HashSet;

fn outcomes() -> [Outcome; 9] {
    let win = |winner, reason| Outcome::Win { winner, reason };
    [
        win(Color::White, WinReason::NoPieces),
        win(Color::White, WinReason::NoMoves),
        win(Color::Black, WinReason::NoPieces),
        win(Color::Black, WinReason::NoMoves),
        Outcome::Draw(DrawReason::Repetition),
        Outcome::Draw(DrawReason::KingsVsKing),
        Outcome::Draw(DrawReason::KingsOnly),
        Outcome::Draw(DrawReason::Balance { pieces: 5, moves: 30 }),
        Outcome::Draw(DrawReason::MainRoad),
    ]
}

fn pdn_errors() -> Vec<PdnError> {
    let bad_move = |token: &str, problem| PdnError::BadMove {
        line: 7,
        token: token.into(),
        problem,
    };
    vec![
        PdnError::TooLarge,
        PdnError::Empty,
        PdnError::NotPdn,
        PdnError::TooManyMoves,
        PdnError::WrongGameType("20".into()),
        PdnError::BadFen("W:Wz9".into()),
        PdnError::BadTag { line: 3 },
        PdnError::UnbalancedVariation { line: 9 },
        PdnError::VariationWithoutMove { line: 11 },
        PdnError::UnterminatedComment { line: 13 },
        bad_move("c3-c4", MoveProblem::Syntax),
        bad_move("c3-d4", MoveProblem::Illegal),
        bad_move("c3:g7", MoveProblem::Ambiguous),
        bad_move("e3-f4", MoveProblem::GameOver),
    ]
}

const fn is_cyrillic(c: char) -> bool {
    matches!(c, '\u{0400}'..='\u{04FF}')
}

const fn is_han(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}')
}

const fn is_kana(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{30FF}')
}

const fn is_hangul(c: char) -> bool {
    matches!(c, '\u{AC00}'..='\u{D7A3}')
}

#[test]
fn there_are_twelve_distinct_languages() {
    assert_eq!(Lang::ALL.len(), 12);
    let codes: HashSet<_> = Lang::ALL.iter().map(|lang| lang.code()).collect();
    let names: HashSet<_> = Lang::ALL.iter().map(|lang| lang.native_name()).collect();
    assert_eq!(codes.len(), 12);
    assert_eq!(names.len(), 12);
    assert!(Lang::ALL.contains(&Lang::DEFAULT) && Lang::ALL.contains(&Lang::FALLBACK));
    for lang in Lang::ALL {
        assert_eq!(lang.code().len(), 2, "{lang:?}");
        assert!(lang.code().chars().all(|c| c.is_ascii_lowercase()), "{lang:?}");
    }
}

#[test]
fn every_text_is_translated_into_every_language() {
    for lang in Lang::ALL {
        for &key in Text::ALL {
            let text = tr(lang, key);
            assert!(!text.trim().is_empty(), "{lang:?} / {key:?}");
            assert_eq!(text, text.trim(), "{lang:?} / {key:?}: stray whitespace");
            assert!(
                !text.contains("  "),
                "{lang:?} / {key:?}: double space in {text:?}"
            );
        }
    }
}

#[test]
fn translations_are_written_in_the_script_of_their_language() {
    // Texts that may consist of an international word only.
    let international = [Text::Ok];
    for lang in Lang::ALL {
        for &key in Text::ALL {
            let text = tr(lang, key);
            let has = |script: fn(char) -> bool| text.chars().any(script);
            let native = match lang {
                Lang::Ru => has(is_cyrillic),
                Lang::Zh => has(is_han) && !has(is_kana) && !has(is_hangul),
                Lang::Ja => has(is_kana) || has(is_han),
                Lang::Ko => has(is_hangul),
                _ => true,
            };
            assert!(
                native || international.contains(&key),
                "{lang:?} / {key:?}: {text:?} is not in the script of the language"
            );
            let foreign = match lang {
                Lang::Ru => has(is_han) || has(is_kana) || has(is_hangul),
                Lang::Zh | Lang::Ja | Lang::Ko => has(is_cyrillic),
                _ => has(is_cyrillic) || has(is_han) || has(is_kana) || has(is_hangul),
            };
            assert!(!foreign, "{lang:?} / {key:?}: {text:?} mixes in a foreign script");
        }
    }
}

#[test]
fn no_language_is_a_copy_of_another() {
    for (index, &first) in Lang::ALL.iter().enumerate() {
        for &second in &Lang::ALL[index + 1..] {
            let same = Text::ALL
                .iter()
                .filter(|&&key| tr(first, key) == tr(second, key))
                .count();
            // Related languages share a few words ("Cancelar", "OK"), but no more.
            assert!(
                same * 5 < Text::ALL.len(),
                "{first:?} and {second:?} share {same} of {} texts",
                Text::ALL.len()
            );
        }
    }
}

#[test]
fn templates_have_the_same_placeholders_in_every_language() {
    let expected: &[(Text, &[&str])] = &[
        (Text::DrawRepetition, &["count"]),
        (Text::DrawKingsVsKing, &["moves"]),
        (Text::DrawKingsOnly, &["moves"]),
        (Text::DrawBalance, &["moves", "pieces"]),
        (Text::DrawMainRoad, &["moves"]),
        (Text::PdnWrongGameType, &["value"]),
        (Text::PdnBadFen, &["value"]),
        (Text::PdnBadTag, &["line"]),
        (Text::PdnUnbalancedVariation, &["line"]),
        (Text::PdnVariationWithoutMove, &["line"]),
        (Text::PdnUnterminatedComment, &["line"]),
        (Text::PdnBadMove, &["line", "problem", "token"]),
    ];
    for lang in Lang::ALL {
        for &key in Text::ALL {
            let mut found = placeholders(tr(lang, key));
            found.sort_unstable();
            let wanted = expected
                .iter()
                .find(|(template, _)| *template == key)
                .map_or(&[][..], |(_, names)| names);
            assert_eq!(found, wanted, "{lang:?} / {key:?}");
        }
    }
}

#[test]
fn fill_replaces_named_placeholders_once() {
    assert_eq!(fill("no placeholders", &[]), "no placeholders");
    assert_eq!(fill("{a}-{b}-{a}", &[("a", &1), ("b", &"two")]), "1-two-1");
    // Braces in the inserted text are not placeholders: file contents cannot smuggle
    // a second substitution in.
    assert_eq!(
        fill("{token} at {line}", &[("token", &"{line}"), ("line", &7)]),
        "{line} at 7"
    );
    // Unknown names and unmatched braces stay as they are.
    assert_eq!(fill("{x} {a} } { {a", &[("a", &"!")]), "{x} ! } { {a");
    assert_eq!(fill("", &[("a", &1)]), "");
    assert_eq!(fill("ünï {a} 码", &[("a", &"值")]), "ünï 值 码");
    assert_eq!(placeholders("{a} and {b}, {a}"), ["a", "b", "a"]);
    assert_eq!(placeholders("none } {"), Vec::<&str>::new());
}

#[test]
fn language_codes_round_trip() {
    for lang in Lang::ALL {
        assert_eq!(Lang::from_code(lang.code()), Some(lang));
        assert_eq!(Lang::from_code(&lang.code().to_uppercase()), Some(lang));
    }
    assert_eq!(Lang::from_code(" EN "), Some(Lang::En));
    assert_eq!(Lang::from_code("xx"), None);
    assert_eq!(Lang::from_code(""), None);
    assert_eq!(Lang::from_code("english"), None);
}

#[test]
fn system_locale_selects_the_language() {
    for (locale, lang) in [
        ("ru_RU.UTF-8", Lang::Ru),
        ("en_US.UTF-8", Lang::En),
        ("en_GB", Lang::En),
        ("de_DE.UTF-8", Lang::De),
        ("de_AT@euro", Lang::De),
        ("fr_CA.UTF-8", Lang::Fr),
        ("es_MX.UTF-8", Lang::Es),
        ("pt_BR.UTF-8", Lang::Pt),
        ("pt_PT", Lang::Pt),
        ("it_IT", Lang::It),
        ("tr_TR.UTF-8", Lang::Tr),
        ("id_ID.UTF-8", Lang::Id),
        ("in_ID", Lang::Id),
        ("zh_CN.UTF-8", Lang::Zh),
        ("zh-Hans", Lang::Zh),
        ("ja_JP.UTF-8", Lang::Ja),
        ("ko_KR.UTF-8", Lang::Ko),
        ("ja", Lang::Ja),
        ("  ko_KR  ", Lang::Ko),
        ("RU_ru", Lang::Ru),
    ] {
        assert_eq!(Lang::from_locale(Some(locale)), lang, "{locale}");
    }
    // A language without a translation falls back to English.
    for locale in ["pl_PL.UTF-8", "uk_UA", "ar_EG.UTF-8", "hi_IN", "xx", "_"] {
        assert_eq!(Lang::from_locale(Some(locale)), Lang::FALLBACK, "{locale}");
    }
    // No locale at all: the default language.
    for locale in [None, Some(""), Some("  "), Some("C"), Some("POSIX"), Some("c")] {
        assert_eq!(Lang::from_locale(locale), Lang::DEFAULT, "{locale:?}");
    }
    assert_eq!(Lang::DEFAULT, Lang::Ru);
    assert_eq!(Lang::FALLBACK, Lang::En);
}

#[test]
fn every_outcome_has_a_title_and_a_reason_in_every_language() {
    for lang in Lang::ALL {
        let mut reasons = HashSet::new();
        for outcome in &outcomes() {
            let reason = outcome_reason(lang, outcome);
            assert!(reason.chars().count() > 5, "{lang:?}: {reason}");
            assert!(
                !reason.contains('{') && !reason.contains('}'),
                "{lang:?}: unfilled placeholder in {reason:?}"
            );
            assert!(reasons.insert(reason), "{lang:?}: two outcomes share a reason");
            for human in [Color::White, Color::Black] {
                assert!(!outcome_title(lang, outcome, human).is_empty());
            }
        }
        let titles: HashSet<_> = [Text::YouWin, Text::YouLose, Text::Draw]
            .into_iter()
            .map(|key| tr(lang, key))
            .collect();
        assert_eq!(titles.len(), 3, "{lang:?}");
    }
}

#[test]
fn draw_reasons_name_the_limits_of_the_rules() {
    for lang in Lang::ALL {
        let reason = |draw| outcome_reason(lang, &Outcome::Draw(draw));
        let has = |text: &str, number: u32| text.contains(&number.to_string());
        assert!(has(&reason(DrawReason::Repetition), REPETITION_LIMIT), "{lang:?}");
        assert!(
            has(&reason(DrawReason::KingsVsKing), KINGS_VS_KING_MOVES),
            "{lang:?}"
        );
        assert!(has(&reason(DrawReason::KingsOnly), KINGS_ONLY_MOVES), "{lang:?}");
        assert!(has(&reason(DrawReason::MainRoad), MAIN_ROAD_MOVES), "{lang:?}");
        let balance = reason(DrawReason::Balance { pieces: 7, moves: 60 });
        assert!(has(&balance, 7) && has(&balance, 60), "{lang:?}: {balance}");
    }
}

#[test]
fn russian_outcomes_read_as_before() {
    let white_wins = Outcome::Win {
        winner: Color::White,
        reason: WinReason::NoMoves,
    };
    assert_eq!(outcome_title(Lang::Ru, &white_wins, Color::White), "Вы выиграли");
    assert_eq!(outcome_title(Lang::Ru, &white_wins, Color::Black), "Вы проиграли");
    assert_eq!(
        outcome_reason(Lang::Ru, &white_wins),
        "У чёрных нет допустимых ходов"
    );
    assert_eq!(
        outcome_reason(Lang::Ru, &Outcome::Draw(DrawReason::KingsVsKing)),
        "Три и более дамки не взяли одинокую дамку за 15 ходов"
    );
    assert_eq!(
        outcome_reason(Lang::En, &Outcome::Draw(DrawReason::MainRoad)),
        "Three pieces failed to capture a lone king on the main road in 5 moves"
    );
    assert_eq!(
        outcome_reason(
            Lang::Ru,
            &Outcome::Draw(DrawReason::Balance { pieces: 5, moves: 30 })
        ),
        "Соотношение сил не менялось 30 ходов (у обеих сторон дамки, фигур на доске: 5)"
    );
}

#[test]
fn every_pdn_error_has_a_message_in_every_language() {
    for lang in Lang::ALL {
        let mut messages = HashSet::new();
        for error in &pdn_errors() {
            let text = pdn_error_text(lang, error);
            assert!(text.chars().count() > 8, "{lang:?} / {error:?}: {text}");
            assert!(
                !text.contains('{') && !text.contains('}'),
                "{lang:?} / {error:?}: unfilled placeholder in {text:?}"
            );
            assert!(
                messages.insert(text),
                "{lang:?} / {error:?}: message repeats another"
            );
        }
    }
}

#[test]
fn pdn_error_messages_carry_the_details() {
    for lang in Lang::ALL {
        let text = |error: &PdnError| pdn_error_text(lang, error);
        assert!(
            text(&PdnError::WrongGameType("20".into())).contains("20"),
            "{lang:?}"
        );
        assert!(
            text(&PdnError::WrongGameType("20".into())).contains("25"),
            "{lang:?}"
        );
        assert!(
            text(&PdnError::BadFen("W:Wz9".into())).contains("W:Wz9"),
            "{lang:?}"
        );
        assert!(text(&PdnError::BadTag { line: 31 }).contains("31"), "{lang:?}");
        assert!(
            text(&PdnError::UnbalancedVariation { line: 32 }).contains("32"),
            "{lang:?}"
        );
        assert!(
            text(&PdnError::VariationWithoutMove { line: 33 }).contains("33"),
            "{lang:?}"
        );
        assert!(
            text(&PdnError::UnterminatedComment { line: 34 }).contains("34"),
            "{lang:?}"
        );
        for (problem, key) in [
            (MoveProblem::Syntax, Text::MoveSyntax),
            (MoveProblem::Illegal, Text::MoveIllegal),
            (MoveProblem::Ambiguous, Text::MoveAmbiguous),
            (MoveProblem::GameOver, Text::MoveGameOver),
        ] {
            let message = text(&PdnError::BadMove {
                line: 35,
                token: "c3-c4".into(),
                problem,
            });
            assert!(
                message.contains("35") && message.contains("c3-c4"),
                "{lang:?}: {message}"
            );
            assert!(message.contains(tr(lang, key)), "{lang:?}: {message}");
        }
        // A token that looks like a placeholder is shown as it is.
        let tricky = text(&PdnError::BadMove {
            line: 36,
            token: "{problem}{line}".into(),
            problem: MoveProblem::Syntax,
        });
        assert!(tricky.contains("{problem}{line}"), "{lang:?}: {tricky}");
    }
    let russian = pdn_error_text(
        Lang::Ru,
        &PdnError::BadMove {
            line: 7,
            token: "c3-c4".into(),
            problem: MoveProblem::Syntax,
        },
    );
    assert_eq!(russian, "Строка 7: «c3-c4» — это не запись хода.");
}

#[test]
fn level_labels_name_the_level() {
    assert_eq!(level_label(Lang::Ru, 1), "1 — новичок");
    assert_eq!(level_label(Lang::En, 10), "10 — maximum");
    assert_eq!(level_label(Lang::De, 4), "4 — Amateur");
    for lang in Lang::ALL {
        let labels: Vec<String> = (1..=10).map(|level| level_label(lang, level)).collect();
        for (index, label) in labels.iter().enumerate() {
            assert!(label.starts_with(&(index + 1).to_string()), "{lang:?}: {label}");
        }
        let distinct: HashSet<_> = labels.iter().collect();
        assert_eq!(distinct.len(), 10, "{lang:?}");
        let names: HashSet<_> = [
            Text::LevelBeginner,
            Text::LevelWeak,
            Text::LevelAmateur,
            Text::LevelSkilled,
            Text::LevelStrong,
            Text::LevelMaximum,
        ]
        .into_iter()
        .map(|key| tr(lang, key))
        .collect();
        assert_eq!(names.len(), 6, "{lang:?}: level names differ from one another");
    }
}

#[test]
fn key_help_lists_the_same_shortcuts_in_every_language() {
    for lang in Lang::ALL {
        // One paragraph that the dialog wraps to its own width.
        let help = tr(lang, Text::KeysHelp);
        assert!(!help.contains('\n'), "{lang:?}");
        for shortcut in ["F9", "End", "+N", "+O", "+S", "Cmd", "macOS"] {
            assert!(help.contains(shortcut), "{lang:?}: {shortcut} is missing");
        }
        // Tooltips name the key that does the same as the button.
        for key in [
            Text::StepBack,
            Text::StepForward,
            Text::ToStart,
            Text::ToEnd,
            Text::ContinueHere,
        ] {
            let text = tr(lang, key);
            assert!(
                text.contains('(') || text.contains('（'),
                "{lang:?} / {key:?}: {text:?} names no key"
            );
        }
    }
}
