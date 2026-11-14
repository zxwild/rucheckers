//! French. A no-break space (`\u{a0}`) precedes colons and pads guillemets, as French
//! typography requires.

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::AppTitle => "Dames russes",
        Text::NewGame => "Nouvelle partie",
        Text::Open => "Ouvrir…",
        Text::Save => "Enregistrer…",
        Text::Settings => "Paramètres",
        Text::Close => "Fermer",
        Text::Cancel => "Annuler",
        Text::Start => "Commencer",
        Text::Ok => "OK",
        Text::YourColor => "Votre couleur",
        Text::White => "Blancs",
        Text::Black => "Noirs",
        Text::Level => "Niveau",
        Text::LevelBeginner => "débutant",
        Text::LevelWeak => "faible",
        Text::LevelAmateur => "amateur",
        Text::LevelSkilled => "confirmé",
        Text::LevelStrong => "fort",
        Text::LevelMaximum => "maximum",
        Text::Language => "Langue",
        Text::Sound => "Son",
        Text::SoundOn => "Son activé",
        Text::SoundOff => "Son désactivé",
        Text::MovesPanel => "Panneau des coups",
        Text::Effects => "Effets spéciaux",
        Text::EffectsEnabled => "Afficher les effets spéciaux",
        Text::EffectLightning => "Éclair derrière le pion lors des prises multiples",
        Text::EffectSparks => "Étincelles des pions capturés",
        Text::EffectFlashes => "Éclats et halo",
        Text::ShowPanel => "Afficher le panneau",
        Text::HidePanel => "Masquer le panneau",
        Text::YourMove => "À vous de jouer",
        Text::MustCapture => "À vous de jouer\u{a0}: la prise est obligatoire",
        Text::Thinking => "L'ordinateur réfléchit…",
        Text::ComputerToMove => "À l'ordinateur de jouer",
        Text::Review => "Consultation",
        Text::ContinueHere => "Continuer d'ici (Espace)",
        Text::ToStart => "Au début (Home)",
        Text::StepBack => "Coup précédent (Flèche gauche)",
        Text::StepForward => "Coup suivant (Flèche droite)",
        Text::ToEnd => "À la fin (End)",
        Text::Variations => "variantes\u{a0}:",
        Text::NoMovesYet => "Aucun coup pour l'instant",
        Text::YouPlayWhite => "Vous jouez les Blancs",
        Text::YouPlayBlack => "Vous jouez les Noirs",
        Text::YouWin => "Vous avez gagné",
        Text::YouLose => "Vous avez perdu",
        Text::Draw => "Partie nulle",
        Text::WhiteNoPieces => "Les Blancs n'ont plus de pièces",
        Text::BlackNoPieces => "Les Noirs n'ont plus de pièces",
        Text::WhiteNoMoves => "Les Blancs n'ont plus de coup légal",
        Text::BlackNoMoves => "Les Noirs n'ont plus de coup légal",
        Text::DrawRepetition => "La même position s'est produite {count} fois",
        Text::DrawKingsVsKing => "Trois dames ou plus n'ont pas capturé une dame isolée en {moves} coups",
        Text::DrawKingsOnly => {
            "{moves} coups ont été joués uniquement par des dames, sans prise ni coup de pion"
        }
        Text::DrawBalance => {
            "Le rapport de forces n'a pas changé pendant {moves} coups \
             (les deux camps ont des dames, pièces sur le damier\u{a0}: {pieces})"
        }
        Text::DrawMainRoad => {
            "Trois pièces n'ont pas capturé une dame isolée sur la grande diagonale en {moves} coups"
        }
        Text::OpenTitle => "Ouvrir une partie",
        Text::SaveTitle => "Enregistrer la partie",
        Text::PdnFilter => "Parties PDN",
        Text::ErrorTitle => "Erreur",
        Text::ReadFailed => "Impossible de lire le fichier",
        Text::WriteFailed => "Impossible d'enregistrer le fichier",
        Text::SettingsNotSaved => "Impossible d'enregistrer les paramètres",
        Text::StartFailed => {
            "Impossible d'ouvrir la fenêtre. Le programme a besoin d'une session graphique et d'un pilote graphique prenant en charge OpenGL 2.0 ou une version plus récente."
        }
        Text::PdnTooLarge => "Le fichier est trop volumineux pour être une partie.",
        Text::PdnEmpty => "Le fichier est vide\u{a0}: il ne contient aucune partie.",
        Text::PdnNotPdn => "Le fichier ne ressemble pas à une partie au format PDN.",
        Text::PdnTooManyMoves => "La partie contient trop de coups.",
        Text::PdnWrongGameType => {
            "Cette partie relève d'une autre variante du jeu de dames \
             (GameType «\u{a0}{value}\u{a0}»). \
             Seules les dames russes (GameType 25) sont prises en charge."
        }
        Text::PdnBadFen => "Position initiale invalide dans la balise FEN\u{a0}: «\u{a0}{value}\u{a0}».",
        Text::PdnBadTag => "Ligne {line}\u{a0}: l'en-tête de la partie est endommagé (balise non fermée).",
        Text::PdnUnbalancedVariation => {
            "Ligne {line}\u{a0}: les parenthèses des variantes ne sont pas équilibrées."
        }
        Text::PdnVariationWithoutMove => {
            "Ligne {line}\u{a0}: une variante commence avant le coup auquel elle se rapporte."
        }
        Text::PdnUnterminatedComment => "Ligne {line}\u{a0}: commentaire non fermé.",
        Text::PdnBadMove => "Ligne {line}\u{a0}: «\u{a0}{token}\u{a0}» — {problem}.",
        Text::MoveSyntax => "ce n'est pas un coup",
        Text::MoveIllegal => "ce coup est impossible dans cette position",
        Text::MoveAmbiguous => "la notation de la prise est ambiguë",
        Text::MoveGameOver => "la partie est déjà terminée à ce stade",
        Text::HumanName => "Joueur",
        Text::ComputerName => "Ordinateur",
        Text::Keys => "Touches",
        Text::KeysHelp => {
            "Flèches gauche et droite — parcourir les coups\u{a0}; \
             haut et bas — variantes\u{a0}; \
             Home et End — début et fin de la partie\u{a0}; \
             Espace — continuer d'ici\u{a0}; \
             Échap ou bouton droit de la souris — annuler la sélection\u{a0}; \
             F9 — panneau des coups\u{a0}; \
             Ctrl+N, Ctrl+O, Ctrl+S (Cmd sur macOS) — nouvelle partie, ouvrir, enregistrer"
        }
    }
}
