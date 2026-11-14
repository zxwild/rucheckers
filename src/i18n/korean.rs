//! Korean.

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::AppTitle => "러시아 체커",
        Text::NewGame => "새 대국",
        Text::Open => "열기…",
        Text::Save => "저장…",
        Text::Settings => "설정",
        Text::Close => "닫기",
        Text::Cancel => "취소",
        Text::Start => "시작",
        Text::Ok => "확인",
        Text::YourColor => "내 색",
        Text::White => "백",
        Text::Black => "흑",
        Text::Level => "난이도",
        Text::LevelBeginner => "입문",
        Text::LevelWeak => "약함",
        Text::LevelAmateur => "아마추어",
        Text::LevelSkilled => "숙련",
        Text::LevelStrong => "강함",
        Text::LevelMaximum => "최강",
        Text::Language => "언어",
        Text::Sound => "소리",
        Text::SoundOn => "소리 켜짐",
        Text::SoundOff => "소리 꺼짐",
        Text::MovesPanel => "기보 패널",
        Text::Effects => "특수 효과",
        Text::EffectsEnabled => "특수 효과 표시",
        Text::EffectLightning => "여러 말을 잡을 때 말 뒤를 따르는 번개",
        Text::EffectSparks => "잡힌 말에서 튀는 불꽃",
        Text::EffectFlashes => "섬광과 빛",
        Text::ShowPanel => "패널 표시",
        Text::HidePanel => "패널 숨기기",
        Text::YourMove => "내 차례입니다",
        Text::MustCapture => "내 차례입니다: 반드시 잡아야 합니다",
        Text::Thinking => "컴퓨터가 생각 중입니다…",
        Text::ComputerToMove => "컴퓨터 차례입니다",
        Text::Review => "복기",
        Text::ContinueHere => "여기서 계속하기 (스페이스)",
        Text::ToStart => "처음으로 (Home)",
        Text::StepBack => "한 수 뒤로 (왼쪽 화살표)",
        Text::StepForward => "한 수 앞으로 (오른쪽 화살표)",
        Text::ToEnd => "끝으로 (End)",
        Text::Variations => "변화:",
        Text::NoMovesYet => "아직 둔 수가 없습니다",
        Text::YouPlayWhite => "백으로 둡니다",
        Text::YouPlayBlack => "흑으로 둡니다",
        Text::YouWin => "이겼습니다",
        Text::YouLose => "졌습니다",
        Text::Draw => "무승부",
        Text::WhiteNoPieces => "백에게 남은 말이 없습니다",
        Text::BlackNoPieces => "흑에게 남은 말이 없습니다",
        Text::WhiteNoMoves => "백이 둘 수 있는 수가 없습니다",
        Text::BlackNoMoves => "흑이 둘 수 있는 수가 없습니다",
        Text::DrawRepetition => "같은 국면이 {count}번 나왔습니다",
        Text::DrawKingsVsKing => "킹 세 개 이상이 {moves}수 안에 상대의 하나뿐인 킹을 잡지 못했습니다",
        Text::DrawKingsOnly => "{moves}수 동안 킹만 움직였고, 잡기도 일반 말의 이동도 없었습니다",
        Text::DrawBalance => {
            "전력의 균형이 {moves}수 동안 바뀌지 않았습니다 \
             (양쪽 모두 킹이 있음, 판 위의 말: {pieces}개)"
        }
        Text::DrawMainRoad => "말 세 개가 {moves}수 안에 큰 대각선 위의 하나뿐인 킹을 잡지 못했습니다",
        Text::OpenTitle => "대국 열기",
        Text::SaveTitle => "대국 저장",
        Text::PdnFilter => "PDN 대국",
        Text::ErrorTitle => "오류",
        Text::ReadFailed => "파일을 읽을 수 없습니다",
        Text::WriteFailed => "파일을 저장할 수 없습니다",
        Text::SettingsNotSaved => "설정을 저장할 수 없습니다",
        Text::StartFailed => {
            "창을 열 수 없습니다. 이 프로그램에는 그래픽 세션과 OpenGL 2.0 이상을 지원하는 그래픽 드라이버가 필요합니다."
        }
        Text::PdnTooLarge => "파일이 너무 커서 대국 기록으로 읽을 수 없습니다.",
        Text::PdnEmpty => "파일이 비어 있습니다: 대국이 없습니다.",
        Text::PdnNotPdn => "PDN 형식의 대국 기록이 아닌 것 같습니다.",
        Text::PdnTooManyMoves => "대국의 수가 너무 많습니다.",
        Text::PdnWrongGameType => {
            "다른 종류의 체커 대국입니다 (GameType “{value}”). \
             러시아 체커(GameType 25)만 지원합니다."
        }
        Text::PdnBadFen => "FEN 태그의 시작 국면이 올바르지 않습니다: “{value}”.",
        Text::PdnBadTag => "{line}번째 줄: 대국 머리말이 손상되었습니다 (태그가 닫히지 않음).",
        Text::PdnUnbalancedVariation => "{line}번째 줄: 변화의 괄호가 짝이 맞지 않습니다.",
        Text::PdnVariationWithoutMove => "{line}번째 줄: 변화가 해당하는 수보다 먼저 시작됩니다.",
        Text::PdnUnterminatedComment => "{line}번째 줄: 주석이 닫히지 않았습니다.",
        Text::PdnBadMove => "{line}번째 줄: “{token}” — {problem}.",
        Text::MoveSyntax => "수가 아닙니다",
        Text::MoveIllegal => "이 국면에서는 둘 수 없는 수입니다",
        Text::MoveAmbiguous => "잡기 표기가 모호합니다",
        Text::MoveGameOver => "이 시점에 대국은 이미 끝났습니다",
        Text::HumanName => "플레이어",
        Text::ComputerName => "컴퓨터",
        Text::Keys => "키",
        Text::KeysHelp => {
            "왼쪽·오른쪽 화살표 — 수 넘기기; \
             위·아래 화살표 — 변화 전환; \
             Home과 End — 대국의 처음과 끝; \
             스페이스 — 여기서 계속하기; \
             Esc 또는 마우스 오른쪽 버튼 — 선택 취소; \
             F9 — 기보 패널; \
             Ctrl+N, Ctrl+O, Ctrl+S (macOS에서는 Cmd) — 새 대국, 열기, 저장"
        }
    }
}
