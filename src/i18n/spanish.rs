//! Spanish.

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::AppTitle => "Damas rusas",
        Text::NewGame => "Nueva partida",
        Text::Open => "Abrir…",
        Text::Save => "Guardar…",
        Text::Settings => "Ajustes",
        Text::Close => "Cerrar",
        Text::Cancel => "Cancelar",
        Text::Start => "Empezar",
        Text::Ok => "Aceptar",
        Text::YourColor => "Tu color",
        Text::White => "Blancas",
        Text::Black => "Negras",
        Text::Level => "Nivel",
        Text::LevelBeginner => "principiante",
        Text::LevelWeak => "flojo",
        Text::LevelAmateur => "aficionado",
        Text::LevelSkilled => "experimentado",
        Text::LevelStrong => "fuerte",
        Text::LevelMaximum => "máximo",
        Text::Language => "Idioma",
        Text::Sound => "Sonido",
        Text::SoundOn => "Sonido activado",
        Text::SoundOff => "Sonido desactivado",
        Text::MovesPanel => "Panel de jugadas",
        Text::Effects => "Efectos especiales",
        Text::EffectsEnabled => "Mostrar efectos especiales",
        Text::EffectLightning => "Rayo tras la ficha en las capturas múltiples",
        Text::EffectSparks => "Chispas de las fichas capturadas",
        Text::EffectFlashes => "Destellos y resplandor",
        Text::ShowPanel => "Mostrar panel",
        Text::HidePanel => "Ocultar panel",
        Text::YourMove => "Te toca jugar",
        Text::MustCapture => "Te toca jugar: la captura es obligatoria",
        Text::Thinking => "La computadora está pensando…",
        Text::ComputerToMove => "Juega la computadora",
        Text::Review => "Revisión",
        Text::ContinueHere => "Continuar desde aquí (Espacio)",
        Text::ToStart => "Al inicio (Home)",
        Text::StepBack => "Jugada anterior (Flecha izquierda)",
        Text::StepForward => "Jugada siguiente (Flecha derecha)",
        Text::ToEnd => "Al final (End)",
        Text::Variations => "variantes:",
        Text::NoMovesYet => "Aún no hay jugadas",
        Text::YouPlayWhite => "Juegas con blancas",
        Text::YouPlayBlack => "Juegas con negras",
        Text::YouWin => "Has ganado",
        Text::YouLose => "Has perdido",
        Text::Draw => "Tablas",
        Text::WhiteNoPieces => "Las blancas se han quedado sin fichas",
        Text::BlackNoPieces => "Las negras se han quedado sin fichas",
        Text::WhiteNoMoves => "Las blancas no tienen jugadas legales",
        Text::BlackNoMoves => "Las negras no tienen jugadas legales",
        Text::DrawRepetition => "La misma posición se ha repetido {count} veces",
        Text::DrawKingsVsKing => "Tres o más damas no han capturado a una dama solitaria en {moves} jugadas",
        Text::DrawKingsOnly => {
            "Se han hecho {moves} jugadas solo con damas, sin capturas ni movimientos de peones"
        }
        Text::DrawBalance => {
            "La relación de fuerzas no ha cambiado en {moves} jugadas \
             (ambos bandos tienen damas, fichas en el tablero: {pieces})"
        }
        Text::DrawMainRoad => {
            "Tres fichas no han capturado a una dama solitaria en la diagonal principal en {moves} jugadas"
        }
        Text::OpenTitle => "Abrir partida",
        Text::SaveTitle => "Guardar partida",
        Text::PdnFilter => "Partidas PDN",
        Text::ErrorTitle => "Error",
        Text::ReadFailed => "No se pudo leer el archivo",
        Text::WriteFailed => "No se pudo guardar el archivo",
        Text::SettingsNotSaved => "No se pudieron guardar los ajustes",
        Text::StartFailed => {
            "No se pudo abrir la ventana. El programa necesita una sesión gráfica y un controlador gráfico compatible con OpenGL 2.0 o posterior."
        }
        Text::PdnTooLarge => "El archivo es demasiado grande para ser una partida.",
        Text::PdnEmpty => "El archivo está vacío: no contiene ninguna partida.",
        Text::PdnNotPdn => "El archivo no parece una partida en formato PDN.",
        Text::PdnTooManyMoves => "La partida tiene demasiadas jugadas.",
        Text::PdnWrongGameType => {
            "Es una partida de otra variante de damas (GameType «{value}»). \
             Solo se admiten las damas rusas (GameType 25)."
        }
        Text::PdnBadFen => "Posición inicial no válida en la etiqueta FEN: «{value}».",
        Text::PdnBadTag => "Línea {line}: el encabezado de la partida está dañado (etiqueta sin cerrar).",
        Text::PdnUnbalancedVariation => {
            "Línea {line}: los paréntesis de las variantes no están equilibrados."
        }
        Text::PdnVariationWithoutMove => {
            "Línea {line}: una variante empieza antes de la jugada a la que se refiere."
        }
        Text::PdnUnterminatedComment => "Línea {line}: comentario sin cerrar.",
        Text::PdnBadMove => "Línea {line}: «{token}» — {problem}.",
        Text::MoveSyntax => "no es una jugada",
        Text::MoveIllegal => "la jugada no es posible en esta posición",
        Text::MoveAmbiguous => "la anotación de la captura es ambigua",
        Text::MoveGameOver => "la partida ya ha terminado en este punto",
        Text::HumanName => "Jugador",
        Text::ComputerName => "Computadora",
        Text::Keys => "Teclas",
        Text::KeysHelp => {
            "Flechas izquierda y derecha — recorrer las jugadas; \
             arriba y abajo — variantes; \
             Home y End — inicio y final de la partida; \
             Espacio — continuar desde aquí; \
             Esc o botón derecho del ratón — cancelar la selección; \
             F9 — panel de jugadas; \
             Ctrl+N, Ctrl+O, Ctrl+S (Cmd en macOS) — nueva partida, abrir, guardar"
        }
    }
}
