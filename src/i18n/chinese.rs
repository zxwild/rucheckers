//! Chinese (Simplified).

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::AppTitle => "俄罗斯跳棋",
        Text::NewGame => "新对局",
        Text::Open => "打开…",
        Text::Save => "保存…",
        Text::Settings => "设置",
        Text::Close => "关闭",
        Text::Cancel => "取消",
        Text::Start => "开始",
        Text::Ok => "确定",
        Text::YourColor => "您的棋色",
        Text::White => "白方",
        Text::Black => "黑方",
        Text::Level => "难度",
        Text::LevelBeginner => "新手",
        Text::LevelWeak => "较弱",
        Text::LevelAmateur => "业余",
        Text::LevelSkilled => "熟练",
        Text::LevelStrong => "较强",
        Text::LevelMaximum => "最强",
        Text::Language => "语言",
        Text::Sound => "声音",
        Text::SoundOn => "声音已开启",
        Text::SoundOff => "声音已关闭",
        Text::MovesPanel => "棋谱面板",
        Text::Effects => "特效",
        Text::EffectsEnabled => "显示特效",
        Text::EffectLightning => "连吃时棋子身后的闪电",
        Text::EffectSparks => "被吃棋子迸出的火花",
        Text::EffectFlashes => "闪光与光晕",
        Text::ShowPanel => "显示面板",
        Text::HidePanel => "隐藏面板",
        Text::YourMove => "轮到您走棋",
        Text::MustCapture => "轮到您走棋：必须吃子",
        Text::Thinking => "电脑正在思考…",
        Text::ComputerToMove => "轮到电脑走棋",
        Text::Review => "回看",
        Text::ContinueHere => "从此处继续（空格键）",
        Text::ToStart => "回到开局（Home）",
        Text::StepBack => "后退一步（左方向键）",
        Text::StepForward => "前进一步（右方向键）",
        Text::ToEnd => "跳到末尾（End）",
        Text::Variations => "变着：",
        Text::NoMovesYet => "暂无着法",
        Text::YouPlayWhite => "您执白",
        Text::YouPlayBlack => "您执黑",
        Text::YouWin => "您赢了",
        Text::YouLose => "您输了",
        Text::Draw => "和棋",
        Text::WhiteNoPieces => "白方已无棋子",
        Text::BlackNoPieces => "黑方已无棋子",
        Text::WhiteNoMoves => "白方无合法着法",
        Text::BlackNoMoves => "黑方无合法着法",
        Text::DrawRepetition => "同一局面出现了 {count} 次",
        Text::DrawKingsVsKing => "三个或更多王棋在 {moves} 步内未能吃掉对方的单个王棋",
        Text::DrawKingsOnly => "连续 {moves} 步只有王棋走动，没有吃子，也没有兵的走动",
        Text::DrawBalance => "双方兵力对比已有 {moves} 步没有变化（双方都有王棋，棋盘上的棋子数：{pieces}）",
        Text::DrawMainRoad => "三个棋子在 {moves} 步内未能吃掉大道上的单个王棋",
        Text::OpenTitle => "打开对局",
        Text::SaveTitle => "保存对局",
        Text::PdnFilter => "PDN 对局",
        Text::ErrorTitle => "错误",
        Text::ReadFailed => "无法读取文件",
        Text::WriteFailed => "无法保存文件",
        Text::SettingsNotSaved => "无法保存设置",
        Text::StartFailed => "无法打开窗口。程序需要图形会话以及支持 OpenGL 2.0 或更高版本的显卡驱动程序。",
        Text::PdnTooLarge => "文件太大，不是对局记录。",
        Text::PdnEmpty => "文件为空：其中没有对局。",
        Text::PdnNotPdn => "文件不像是 PDN 格式的对局记录。",
        Text::PdnTooManyMoves => "对局中的着法太多。",
        Text::PdnWrongGameType => {
            "这是另一种跳棋的对局（GameType“{value}”）。仅支持俄罗斯跳棋（GameType 25）。"
        }
        Text::PdnBadFen => "FEN 标签中的初始局面无效：“{value}”。",
        Text::PdnBadTag => "第 {line} 行：对局头部已损坏（标签未闭合）。",
        Text::PdnUnbalancedVariation => "第 {line} 行：变着的括号不配对。",
        Text::PdnVariationWithoutMove => "第 {line} 行：变着出现在它所对应的着法之前。",
        Text::PdnUnterminatedComment => "第 {line} 行：注释未闭合。",
        Text::PdnBadMove => "第 {line} 行：“{token}”——{problem}。",
        Text::MoveSyntax => "这不是着法",
        Text::MoveIllegal => "此着法在当前局面下不可行",
        Text::MoveAmbiguous => "吃子记录有歧义",
        Text::MoveGameOver => "对局此时已经结束",
        Text::HumanName => "玩家",
        Text::ComputerName => "电脑",
        Text::Keys => "按键",
        Text::KeysHelp => {
            "左右方向键——浏览着法；\
             上下方向键——切换变着；\
             Home 和 End——对局的开头和结尾；\
             空格键——从此处继续；\
             Esc 或鼠标右键——取消选择；\
             F9——棋谱面板；\
             Ctrl+N、Ctrl+O、Ctrl+S（macOS 上为 Cmd）——新对局、打开、保存"
        }
    }
}
