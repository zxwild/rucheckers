//! German.

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::YourHome => "Ihr Haus",
        Text::HomeLeft => "Links",
        Text::HomeRight => "Rechts",
        Text::GameRules => "Regeln",
        Text::GameMode => "Spiel",
        Text::Ugolki => "Ecken (Ugolki)",
        Text::GameAppliesNew => "Die Spielwahl gilt ab der nächsten neuen Partie.",
        Text::UgolkiHint => "Bringen Sie alle 12 Steine ins gegenüberliegende Haus. Schritte oder Sprungketten nur waagerecht und senkrecht, ohne Schlagen. Wählen Sie das Zielfeld. Das eigene Haus muss nach 40 Zügen leer sein; Grenze: 80 Züge. Schwarz darf zuletzt antworten. Gegnerische Steine dürfen nicht eingesperrt werden.",
        Text::UgolkiHomeFilled => "Alle Steine haben das gegenüberliegende Haus erreicht",
        Text::UgolkiHomeDeadline => "Das eigene Haus wurde nicht innerhalb von 40 Zügen geräumt",
        Text::UgolkiBothHomeDeadline => "Nach 40 Zügen stehen noch eigene Steine in beiden Häusern",
        Text::UgolkiMoveLimit => "80-Züge-Grenze: Steine im gegenüberliegenden Haus entscheiden",
        Text::UgolkiSimultaneous => "Beide Seiten erreichten das Ziel nach gleich vielen Zügen",
        Text::UgolkiBlocked => "Keine legalen Züge: remis",
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
        Text::Board => "Brett",
        Text::BoardStandard => "Standard",
        Text::BoardMapleWalnut => "Ahorn und Nussbaum",
        Text::BoardBeechMahogany => "Buche und Mahagoni",
        Text::BoardOak => "Eiche und Räuchereiche",
        Text::BoardAshCherry => "Esche und Kirsche, Parkett",
        Text::BoardAmberBirch => "Birke unter Bernsteinlack",
        Text::PlayStyle => "Ecken (Ugolki): Spielstil des Computers",
        Text::BlockHomeExit => "Den Ausgang aus Ihrem Haus versperren",
        Text::BlockHomeExitHint => "Der Computer umstellt Ihre Steine, die noch im Haus stehen, damit sie es bis zum 40. Zug nicht verlassen können. Die Regeln erlauben das, solange die Steine im Haus noch ziehen können.",
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
        Text::StartFailed => {
            "Das Fenster konnte nicht geöffnet werden. Das Programm benötigt eine grafische Sitzung und einen Grafiktreiber mit OpenGL 2.0 oder neuer."
        }
        Text::PdnTooLarge => "Die Datei ist zu groß für eine Partieaufzeichnung.",
        Text::PdnEmpty => "Die Datei ist leer: Sie enthält keine Partie.",
        Text::PdnNotPdn => "Die Datei sieht nicht wie eine Partie im PDN-Format aus.",
        Text::PdnTooManyMoves => "Die Partie enthält zu viele Züge.",
        Text::PdnWrongGameType => "Nicht unterstützte Spielart: {value}. Unterstützt werden Russisch Dame (GameType 25) und Ugolki.",
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
