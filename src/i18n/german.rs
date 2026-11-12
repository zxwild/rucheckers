//! German.

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::AppTitle => "Russische Dame",
        Text::NewGame => "Neue Partie",
        Text::Open => "Öffnen…",
        Text::Save => "Speichern…",
        Text::Settings => "Einstellungen",
        Text::Close => "Schließen",
        Text::Cancel => "Abbrechen",
        Text::Start => "Starten",
        Text::Ok => "OK",
        Text::YourColor => "Ihre Farbe",
        Text::White => "Weiß",
        Text::Black => "Schwarz",
        Text::Level => "Stufe",
        Text::LevelBeginner => "Anfänger",
        Text::LevelWeak => "schwach",
        Text::LevelAmateur => "Amateur",
        Text::LevelSkilled => "erfahren",
        Text::LevelStrong => "stark",
        Text::LevelMaximum => "Maximum",
        Text::Language => "Sprache",
        Text::Sound => "Ton",
        Text::SoundOn => "Ton an",
        Text::SoundOff => "Ton aus",
        Text::MovesPanel => "Zugliste",
        Text::Effects => "Spezialeffekte",
        Text::EffectsEnabled => "Spezialeffekte anzeigen",
        Text::EffectLightning => "Blitz hinter dem Stein bei Mehrfachschlägen",
        Text::EffectSparks => "Funken von geschlagenen Steinen",
        Text::EffectFlashes => "Aufblitzen und Leuchten",
        Text::ShowPanel => "Zugliste einblenden",
        Text::HidePanel => "Zugliste ausblenden",
        Text::YourMove => "Sie sind am Zug",
        Text::MustCapture => "Sie sind am Zug: Es herrscht Schlagzwang",
        Text::Thinking => "Der Computer denkt nach…",
        Text::ComputerToMove => "Der Computer ist am Zug",
        Text::Review => "Ansicht",
        Text::ContinueHere => "Von hier weiterspielen (Leertaste)",
        Text::ToStart => "Zum Anfang (Pos1)",
        Text::StepBack => "Zug zurück (Pfeil links)",
        Text::StepForward => "Zug vor (Pfeil rechts)",
        Text::ToEnd => "Zum Ende (Ende)",
        Text::Variations => "Varianten:",
        Text::NoMovesYet => "Noch keine Züge",
        Text::YouPlayWhite => "Sie spielen Weiß",
        Text::YouPlayBlack => "Sie spielen Schwarz",
        Text::YouWin => "Sie haben gewonnen",
        Text::YouLose => "Sie haben verloren",
        Text::Draw => "Remis",
        Text::WhiteNoPieces => "Weiß hat keine Steine mehr",
        Text::BlackNoPieces => "Schwarz hat keine Steine mehr",
        Text::WhiteNoMoves => "Weiß hat keinen gültigen Zug mehr",
        Text::BlackNoMoves => "Schwarz hat keinen gültigen Zug mehr",
        Text::DrawRepetition => "Dieselbe Stellung ist {count}-mal entstanden",
        Text::DrawKingsVsKing => {
            "Drei oder mehr Damen haben eine einzelne Dame in {moves} Zügen nicht geschlagen"
        }
        Text::DrawKingsOnly => {
            "{moves} Züge lang zogen nur Damen, ohne Schlagen und ohne Züge einfacher Steine"
        }
        Text::DrawBalance => {
            "Das Kräfteverhältnis hat sich {moves} Züge lang nicht geändert \
             (beide Seiten haben Damen, Steine auf dem Brett: {pieces})"
        }
        Text::DrawMainRoad => {
            "Drei Steine haben eine einzelne Dame auf der Hauptdiagonale in {moves} Zügen nicht geschlagen"
        }
        Text::OpenTitle => "Partie öffnen",
        Text::SaveTitle => "Partie speichern",
        Text::PdnFilter => "PDN-Partien",
        Text::ErrorTitle => "Fehler",
        Text::ReadFailed => "Die Datei konnte nicht gelesen werden",
        Text::WriteFailed => "Die Datei konnte nicht gespeichert werden",
        Text::SettingsNotSaved => "Die Einstellungen konnten nicht gespeichert werden",
        Text::PdnTooLarge => "Die Datei ist zu groß für eine Partieaufzeichnung.",
        Text::PdnEmpty => "Die Datei ist leer: Sie enthält keine Partie.",
        Text::PdnNotPdn => "Die Datei sieht nicht wie eine Partie im PDN-Format aus.",
        Text::PdnTooManyMoves => "Die Partie enthält zu viele Züge.",
        Text::PdnWrongGameType => {
            "Dies ist eine Partie einer anderen Damevariante (GameType „{value}“). \
             Unterstützt wird nur Russische Dame (GameType 25)."
        }
        Text::PdnBadFen => "Ungültige Ausgangsstellung im FEN-Tag: „{value}“.",
        Text::PdnBadTag => "Zeile {line}: Der Partiekopf ist beschädigt (Tag nicht geschlossen).",
        Text::PdnUnbalancedVariation => "Zeile {line}: Die Klammern der Varianten sind nicht ausgeglichen.",
        Text::PdnVariationWithoutMove => {
            "Zeile {line}: Eine Variante beginnt vor dem Zug, auf den sie sich bezieht."
        }
        Text::PdnUnterminatedComment => "Zeile {line}: Der Kommentar ist nicht geschlossen.",
        Text::PdnBadMove => "Zeile {line}: „{token}“ — {problem}.",
        Text::MoveSyntax => "das ist kein Zug",
        Text::MoveIllegal => "der Zug ist in dieser Stellung nicht möglich",
        Text::MoveAmbiguous => "die Notation des Schlagzugs ist mehrdeutig",
        Text::MoveGameOver => "die Partie ist an dieser Stelle bereits beendet",
        Text::HumanName => "Spieler",
        Text::ComputerName => "Computer",
        Text::Keys => "Tasten",
        Text::KeysHelp => {
            "Pfeil links und rechts — durch die Züge; \
             Pfeil auf und ab — Varianten; \
             Pos1 und Ende — Anfang und Ende der Partie; \
             Leertaste — von hier weiterspielen; \
             Esc oder rechte Maustaste — Auswahl aufheben; \
             F9 — Zugliste; \
             Strg+N, Strg+O, Strg+S (auf macOS: Cmd) — neue Partie, öffnen, speichern"
        }
    }
}
