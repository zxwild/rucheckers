//! Japanese.

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::AppTitle => "ロシアンチェッカー",
        Text::NewGame => "新しい対局",
        Text::Open => "開く…",
        Text::Save => "保存…",
        Text::Settings => "設定",
        Text::Close => "閉じる",
        Text::Cancel => "キャンセル",
        Text::Start => "開始",
        Text::Ok => "OK",
        Text::YourColor => "あなたの色",
        Text::White => "白",
        Text::Black => "黒",
        Text::Level => "レベル",
        Text::LevelBeginner => "初心者",
        Text::LevelWeak => "弱い",
        Text::LevelAmateur => "アマチュア",
        Text::LevelSkilled => "熟練",
        Text::LevelStrong => "強い",
        Text::LevelMaximum => "最強",
        Text::Language => "言語",
        Text::Sound => "サウンド",
        Text::SoundOn => "サウンド：オン",
        Text::SoundOff => "サウンド：オフ",
        Text::MovesPanel => "棋譜パネル",
        Text::Effects => "特殊効果",
        Text::EffectsEnabled => "特殊効果を表示する",
        Text::EffectLightning => "連続で取るときに駒の後ろを走る稲妻",
        Text::EffectSparks => "取られた駒から飛び散る火花",
        Text::EffectFlashes => "閃光と輝き",
        Text::ShowPanel => "パネルを表示",
        Text::HidePanel => "パネルを隠す",
        Text::YourMove => "あなたの番です",
        Text::MustCapture => "あなたの番です：必ず取らなければなりません",
        Text::Thinking => "コンピューターが考えています…",
        Text::ComputerToMove => "コンピューターの番です",
        Text::Review => "閲覧",
        Text::ContinueHere => "ここから続ける（スペース）",
        Text::ToStart => "最初へ（Home）",
        Text::StepBack => "一手戻る（左矢印）",
        Text::StepForward => "一手進む（右矢印）",
        Text::ToEnd => "最後へ（End）",
        Text::Variations => "変化：",
        Text::NoMovesYet => "まだ手がありません",
        Text::YouPlayWhite => "あなたは白です",
        Text::YouPlayBlack => "あなたは黒です",
        Text::YouWin => "あなたの勝ちです",
        Text::YouLose => "あなたの負けです",
        Text::Draw => "引き分け",
        Text::WhiteNoPieces => "白の駒がなくなりました",
        Text::BlackNoPieces => "黒の駒がなくなりました",
        Text::WhiteNoMoves => "白には指せる手がありません",
        Text::BlackNoMoves => "黒には指せる手がありません",
        Text::DrawRepetition => "同じ局面が {count} 回現れました",
        Text::DrawKingsVsKing => "3 個以上のキングが {moves} 手以内に 1 個だけのキングを取れませんでした",
        Text::DrawKingsOnly => {
            "{moves} 手のあいだキングだけが動き、駒取りも普通の駒の移動もありませんでした"
        }
        Text::DrawBalance => {
            "戦力のバランスが {moves} 手のあいだ変わりませんでした（両者にキングがあり、盤上の駒数：{pieces}）"
        }
        Text::DrawMainRoad => "3 個の駒が {moves} 手以内に大斜線上の 1 個だけのキングを取れませんでした",
        Text::OpenTitle => "対局を開く",
        Text::SaveTitle => "対局を保存",
        Text::PdnFilter => "PDN 対局",
        Text::ErrorTitle => "エラー",
        Text::ReadFailed => "ファイルを読み込めませんでした",
        Text::WriteFailed => "ファイルを保存できませんでした",
        Text::SettingsNotSaved => "設定を保存できませんでした",
        Text::PdnTooLarge => "ファイルが大きすぎて、対局の記録として読み込めません。",
        Text::PdnEmpty => "ファイルは空です：対局が含まれていません。",
        Text::PdnNotPdn => "ファイルは PDN 形式の対局記録ではないようです。",
        Text::PdnTooManyMoves => "対局の手数が多すぎます。",
        Text::PdnWrongGameType => {
            "これは別の種類のチェッカーの対局です（GameType「{value}」）。\
             対応しているのはロシアンチェッカー（GameType 25）だけです。"
        }
        Text::PdnBadFen => "FEN タグの初期局面が正しくありません：「{value}」。",
        Text::PdnBadTag => "{line} 行目：対局のヘッダーが壊れています（タグが閉じられていません）。",
        Text::PdnUnbalancedVariation => "{line} 行目：変化の括弧が対応していません。",
        Text::PdnVariationWithoutMove => "{line} 行目：変化が、対応する手より前に始まっています。",
        Text::PdnUnterminatedComment => "{line} 行目：コメントが閉じられていません。",
        Text::PdnBadMove => "{line} 行目：「{token}」— {problem}。",
        Text::MoveSyntax => "これは手ではありません",
        Text::MoveIllegal => "この局面ではその手は指せません",
        Text::MoveAmbiguous => "駒取りの表記があいまいです",
        Text::MoveGameOver => "この時点で対局はすでに終わっています",
        Text::HumanName => "プレイヤー",
        Text::ComputerName => "コンピューター",
        Text::Keys => "キー操作",
        Text::KeysHelp => {
            "左右の矢印 — 手を進める・戻す。\
             上下の矢印 — 変化の切り替え。\
             Home と End — 対局の最初と最後。\
             スペース — ここから続ける。\
             Esc またはマウスの右ボタン — 選択を取り消す。\
             F9 — 棋譜パネル。\
             Ctrl+N、Ctrl+O、Ctrl+S（macOS では Cmd）— 新しい対局、開く、保存"
        }
    }
}
