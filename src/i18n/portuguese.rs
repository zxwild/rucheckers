//! Portuguese (Brazilian usage).

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::YourHome => "Sua casa",
        Text::HomeLeft => "À esquerda",
        Text::HomeRight => "À direita",
        Text::GameRules => "Regras",
        Text::GameMode => "Jogo",
        Text::Ugolki => "Cantos (Ugolki)",
        Text::GameAppliesNew => "A escolha aplica-se ao iniciar uma nova partida.",
        Text::UgolkiHint => "Leve as 12 peças à casa oposta. Passos ou sequências de saltos apenas na horizontal e vertical, sem capturas. Escolha a casa final. Esvazie a sua casa em 40 jogadas; limite: 80. As pretas têm a última resposta. É proibido bloquear peças adversárias.",
        Text::UgolkiHomeFilled => "Todas as peças chegaram à casa oposta",
        Text::UgolkiHomeDeadline => "A casa inicial não foi esvaziada em 40 jogadas",
        Text::UgolkiBothHomeDeadline => "Ambas as casas ainda têm peças próprias após 40 jogadas",
        Text::UgolkiMoveLimit => "Limite de 80 jogadas: decide o número de peças na casa oposta",
        Text::UgolkiSimultaneous => "Ambos chegaram no mesmo número de jogadas",
        Text::UgolkiBlocked => "Sem movimentos legais: empate",
        Text::AppTitle => "Damas russas",
        Text::NewGame => "Nova partida",
        Text::Open => "Abrir…",
        Text::Save => "Salvar…",
        Text::Settings => "Configurações",
        Text::Close => "Fechar",
        Text::Cancel => "Cancelar",
        Text::Start => "Iniciar",
        Text::Ok => "OK",
        Text::YourColor => "Sua cor",
        Text::White => "Brancas",
        Text::Black => "Pretas",
        Text::Level => "Nível",
        Text::LevelBeginner => "iniciante",
        Text::LevelWeak => "fraco",
        Text::LevelAmateur => "amador",
        Text::LevelSkilled => "experiente",
        Text::LevelStrong => "forte",
        Text::LevelMaximum => "máximo",
        Text::Language => "Idioma",
        Text::Sound => "Som",
        Text::SoundOn => "Som ligado",
        Text::SoundOff => "Som desligado",
        Text::MovesPanel => "Painel de lances",
        Text::Effects => "Efeitos especiais",
        Text::EffectsEnabled => "Mostrar efeitos especiais",
        Text::EffectLightning => "Raio atrás da peça nas capturas múltiplas",
        Text::EffectSparks => "Faíscas das peças capturadas",
        Text::EffectFlashes => "Clarões e brilho",
        Text::Board => "Tabuleiro",
        Text::BoardStandard => "Padrão",
        Text::BoardMapleWalnut => "Bordo e nogueira",
        Text::BoardBeechMahogany => "Faia e mogno",
        Text::BoardOak => "Carvalho e carvalho defumado",
        Text::BoardAshCherry => "Freixo e cerejeira, parquê",
        Text::BoardAmberBirch => "Bétula com verniz âmbar",
        Text::PlayStyle => "Cantos (Ugolki): estilo de jogo do computador",
        Text::BlockHomeExit => "Bloquear a saída da sua casa",
        Text::BlockHomeExitHint => "O computador cerca as suas peças que ainda estão em casa para que não consigam sair até a 40ª jogada. As regras permitem isso enquanto as peças ainda puderem se mover dentro da casa.",
        Text::ShowPanel => "Mostrar painel",
        Text::HidePanel => "Ocultar painel",
        Text::YourMove => "Sua vez",
        Text::MustCapture => "Sua vez: a captura é obrigatória",
        Text::Thinking => "O computador está pensando…",
        Text::ComputerToMove => "Vez do computador",
        Text::Review => "Revisão",
        Text::ContinueHere => "Continuar daqui (Espaço)",
        Text::ToStart => "Para o início (Home)",
        Text::StepBack => "Lance anterior (Seta esquerda)",
        Text::StepForward => "Próximo lance (Seta direita)",
        Text::ToEnd => "Para o fim (End)",
        Text::Variations => "variantes:",
        Text::NoMovesYet => "Ainda não há lances",
        Text::YouPlayWhite => "Você joga com as brancas",
        Text::YouPlayBlack => "Você joga com as pretas",
        Text::YouWin => "Você venceu",
        Text::YouLose => "Você perdeu",
        Text::Draw => "Empate",
        Text::WhiteNoPieces => "As brancas ficaram sem peças",
        Text::BlackNoPieces => "As pretas ficaram sem peças",
        Text::WhiteNoMoves => "As brancas não têm lances válidos",
        Text::BlackNoMoves => "As pretas não têm lances válidos",
        Text::DrawRepetition => "A mesma posição ocorreu {count} vezes",
        Text::DrawKingsVsKing => "Três ou mais damas não capturaram uma dama solitária em {moves} lances",
        Text::DrawKingsOnly => {
            "{moves} lances foram feitos apenas com damas, sem capturas nem lances de pedras"
        }
        Text::DrawBalance => {
            "A relação de forças não mudou durante {moves} lances \
             (os dois lados têm damas, peças no tabuleiro: {pieces})"
        }
        Text::DrawMainRoad => {
            "Três peças não capturaram uma dama solitária na diagonal principal em {moves} lances"
        }
        Text::OpenTitle => "Abrir partida",
        Text::SaveTitle => "Salvar partida",
        Text::PdnFilter => "Partidas PDN",
        Text::ErrorTitle => "Erro",
        Text::ReadFailed => "Não foi possível ler o arquivo",
        Text::WriteFailed => "Não foi possível salvar o arquivo",
        Text::SettingsNotSaved => "Não foi possível salvar as configurações",
        Text::StartFailed => {
            "Não foi possível abrir a janela. O programa precisa de uma sessão gráfica e de um driver de vídeo com suporte a OpenGL 2.0 ou mais recente."
        }
        Text::PdnTooLarge => "O arquivo é grande demais para ser uma partida.",
        Text::PdnEmpty => "O arquivo está vazio: não contém nenhuma partida.",
        Text::PdnNotPdn => "O arquivo não parece uma partida no formato PDN.",
        Text::PdnTooManyMoves => "A partida tem lances demais.",
        Text::PdnWrongGameType => "Tipo de jogo não suportado: {value}. São aceitos damas russas (GameType 25) e Ugolki.",
        Text::PdnBadFen => "Posição inicial inválida na tag FEN: “{value}”.",
        Text::PdnBadTag => "Linha {line}: o cabeçalho da partida está danificado (tag não fechada).",
        Text::PdnUnbalancedVariation => "Linha {line}: os parênteses das variantes não estão balanceados.",
        Text::PdnVariationWithoutMove => "Linha {line}: uma variante começa antes do lance a que se refere.",
        Text::PdnUnterminatedComment => "Linha {line}: comentário não fechado.",
        Text::PdnBadMove => "Linha {line}: “{token}” — {problem}.",
        Text::MoveSyntax => "isto não é um lance",
        Text::MoveIllegal => "o lance não é possível nesta posição",
        Text::MoveAmbiguous => "a notação da captura é ambígua",
        Text::MoveGameOver => "a partida já terminou neste ponto",
        Text::HumanName => "Jogador",
        Text::ComputerName => "Computador",
        Text::Keys => "Teclas",
        Text::KeysHelp => {
            "Setas esquerda e direita — percorrer os lances; \
             para cima e para baixo — variantes; \
             Home e End — início e fim da partida; \
             Espaço — continuar daqui; \
             Esc ou botão direito do mouse — cancelar a seleção; \
             F9 — painel de lances; \
             Ctrl+N, Ctrl+O, Ctrl+S (Cmd no macOS) — nova partida, abrir, salvar"
        }
    }
}
