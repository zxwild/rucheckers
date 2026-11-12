//! Italian.

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::AppTitle => "Dama russa",
        Text::NewGame => "Nuova partita",
        Text::Open => "Apri…",
        Text::Save => "Salva…",
        Text::Settings => "Impostazioni",
        Text::Close => "Chiudi",
        Text::Cancel => "Annulla",
        Text::Start => "Inizia",
        Text::Ok => "OK",
        Text::YourColor => "Il tuo colore",
        Text::White => "Bianco",
        Text::Black => "Nero",
        Text::Level => "Livello",
        Text::LevelBeginner => "principiante",
        Text::LevelWeak => "debole",
        Text::LevelAmateur => "dilettante",
        Text::LevelSkilled => "esperto",
        Text::LevelStrong => "forte",
        Text::LevelMaximum => "massimo",
        Text::Language => "Lingua",
        Text::Sound => "Audio",
        Text::SoundOn => "Audio attivato",
        Text::SoundOff => "Audio disattivato",
        Text::MovesPanel => "Pannello delle mosse",
        Text::Effects => "Effetti speciali",
        Text::EffectsEnabled => "Mostra gli effetti speciali",
        Text::EffectLightning => "Fulmine dietro la pedina nelle prese multiple",
        Text::EffectSparks => "Scintille dalle pedine catturate",
        Text::EffectFlashes => "Lampi e bagliore",
        Text::ShowPanel => "Mostra il pannello",
        Text::HidePanel => "Nascondi il pannello",
        Text::YourMove => "Tocca a te",
        Text::MustCapture => "Tocca a te: la presa è obbligatoria",
        Text::Thinking => "Il computer sta pensando…",
        Text::ComputerToMove => "Tocca al computer",
        Text::Review => "Revisione",
        Text::ContinueHere => "Continua da qui (Spazio)",
        Text::ToStart => "All'inizio (Home)",
        Text::StepBack => "Mossa precedente (Freccia sinistra)",
        Text::StepForward => "Mossa successiva (Freccia destra)",
        Text::ToEnd => "Alla fine (End)",
        Text::Variations => "varianti:",
        Text::NoMovesYet => "Ancora nessuna mossa",
        Text::YouPlayWhite => "Giochi con il Bianco",
        Text::YouPlayBlack => "Giochi con il Nero",
        Text::YouWin => "Hai vinto",
        Text::YouLose => "Hai perso",
        Text::Draw => "Patta",
        Text::WhiteNoPieces => "Il Bianco non ha più pezzi",
        Text::BlackNoPieces => "Il Nero non ha più pezzi",
        Text::WhiteNoMoves => "Il Bianco non ha mosse legali",
        Text::BlackNoMoves => "Il Nero non ha mosse legali",
        Text::DrawRepetition => "La stessa posizione si è verificata {count} volte",
        Text::DrawKingsVsKing => "Tre o più dame non hanno catturato una dama isolata in {moves} mosse",
        Text::DrawKingsOnly => {
            "{moves} mosse sono state giocate solo con le dame, senza prese né mosse di pedina"
        }
        Text::DrawBalance => {
            "Il rapporto di forze non è cambiato per {moves} mosse \
             (entrambi i giocatori hanno dame, pezzi sulla damiera: {pieces})"
        }
        Text::DrawMainRoad => {
            "Tre pezzi non hanno catturato una dama isolata sulla diagonale principale in {moves} mosse"
        }
        Text::OpenTitle => "Apri partita",
        Text::SaveTitle => "Salva partita",
        Text::PdnFilter => "Partite PDN",
        Text::ErrorTitle => "Errore",
        Text::ReadFailed => "Impossibile leggere il file",
        Text::WriteFailed => "Impossibile salvare il file",
        Text::SettingsNotSaved => "Impossibile salvare le impostazioni",
        Text::PdnTooLarge => "Il file è troppo grande per essere una partita.",
        Text::PdnEmpty => "Il file è vuoto: non contiene alcuna partita.",
        Text::PdnNotPdn => "Il file non sembra una partita in formato PDN.",
        Text::PdnTooManyMoves => "La partita contiene troppe mosse.",
        Text::PdnWrongGameType => {
            "È una partita di un'altra variante della dama (GameType «{value}»). \
             È supportata solo la dama russa (GameType 25)."
        }
        Text::PdnBadFen => "Posizione iniziale non valida nel tag FEN: «{value}».",
        Text::PdnBadTag => "Riga {line}: l'intestazione della partita è danneggiata (tag non chiuso).",
        Text::PdnUnbalancedVariation => "Riga {line}: le parentesi delle varianti non sono bilanciate.",
        Text::PdnVariationWithoutMove => {
            "Riga {line}: una variante inizia prima della mossa a cui si riferisce."
        }
        Text::PdnUnterminatedComment => "Riga {line}: commento non chiuso.",
        Text::PdnBadMove => "Riga {line}: «{token}» — {problem}.",
        Text::MoveSyntax => "non è una mossa",
        Text::MoveIllegal => "la mossa non è possibile in questa posizione",
        Text::MoveAmbiguous => "la notazione della presa è ambigua",
        Text::MoveGameOver => "a questo punto la partita è già finita",
        Text::HumanName => "Giocatore",
        Text::ComputerName => "Computer",
        Text::Keys => "Tasti",
        Text::KeysHelp => {
            "Frecce sinistra e destra — scorrere le mosse; \
             su e giù — varianti; \
             Home e End — inizio e fine della partita; \
             Spazio — continua da qui; \
             Esc o tasto destro del mouse — annulla la selezione; \
             F9 — pannello delle mosse; \
             Ctrl+N, Ctrl+O, Ctrl+S (Cmd su macOS) — nuova partita, apri, salva"
        }
    }
}
