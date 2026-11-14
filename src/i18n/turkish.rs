//! Turkish.

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::AppTitle => "Rus Daması",
        Text::NewGame => "Yeni oyun",
        Text::Open => "Aç…",
        Text::Save => "Kaydet…",
        Text::Settings => "Ayarlar",
        Text::Close => "Kapat",
        Text::Cancel => "İptal",
        Text::Start => "Başla",
        Text::Ok => "Tamam",
        Text::YourColor => "Renginiz",
        Text::White => "Beyaz",
        Text::Black => "Siyah",
        Text::Level => "Seviye",
        Text::LevelBeginner => "acemi",
        Text::LevelWeak => "zayıf",
        Text::LevelAmateur => "amatör",
        Text::LevelSkilled => "deneyimli",
        Text::LevelStrong => "güçlü",
        Text::LevelMaximum => "en yüksek",
        Text::Language => "Dil",
        Text::Sound => "Ses",
        Text::SoundOn => "Ses açık",
        Text::SoundOff => "Ses kapalı",
        Text::MovesPanel => "Hamle paneli",
        Text::Effects => "Özel efektler",
        Text::EffectsEnabled => "Özel efektleri göster",
        Text::EffectLightning => "Çoklu taş almada taşın ardından şimşek",
        Text::EffectSparks => "Alınan taşlardan kıvılcımlar",
        Text::EffectFlashes => "Parlamalar ve ışıltı",
        Text::ShowPanel => "Paneli göster",
        Text::HidePanel => "Paneli gizle",
        Text::YourMove => "Sıra sizde",
        Text::MustCapture => "Sıra sizde: taş almak zorunlu",
        Text::Thinking => "Bilgisayar düşünüyor…",
        Text::ComputerToMove => "Sıra bilgisayarda",
        Text::Review => "İnceleme",
        Text::ContinueHere => "Buradan devam et (Boşluk)",
        Text::ToStart => "Başa git (Home)",
        Text::StepBack => "Bir hamle geri (Sol ok)",
        Text::StepForward => "Bir hamle ileri (Sağ ok)",
        Text::ToEnd => "Sona git (End)",
        Text::Variations => "varyantlar:",
        Text::NoMovesYet => "Henüz hamle yok",
        Text::YouPlayWhite => "Beyazla oynuyorsunuz",
        Text::YouPlayBlack => "Siyahla oynuyorsunuz",
        Text::YouWin => "Kazandınız",
        Text::YouLose => "Kaybettiniz",
        Text::Draw => "Beraberlik",
        Text::WhiteNoPieces => "Beyazın taşı kalmadı",
        Text::BlackNoPieces => "Siyahın taşı kalmadı",
        Text::WhiteNoMoves => "Beyazın yapabileceği hamle yok",
        Text::BlackNoMoves => "Siyahın yapabileceği hamle yok",
        Text::DrawRepetition => "Aynı konum {count} kez oluştu",
        Text::DrawKingsVsKing => "Üç veya daha fazla dama, tek kalan damayı {moves} hamlede alamadı",
        Text::DrawKingsOnly => {
            "{moves} hamle boyunca yalnızca damalar oynadı; taş alınmadı ve piyon oynanmadı"
        }
        Text::DrawBalance => {
            "Güç dengesi {moves} hamle boyunca değişmedi \
             (iki tarafta da dama var, tahtadaki taş sayısı: {pieces})"
        }
        Text::DrawMainRoad => "Üç taş, ana köşegendeki tek damayı {moves} hamlede alamadı",
        Text::OpenTitle => "Oyun aç",
        Text::SaveTitle => "Oyunu kaydet",
        Text::PdnFilter => "PDN oyunları",
        Text::ErrorTitle => "Hata",
        Text::ReadFailed => "Dosya okunamadı",
        Text::WriteFailed => "Dosya kaydedilemedi",
        Text::SettingsNotSaved => "Ayarlar kaydedilemedi",
        Text::StartFailed => {
            "Pencere açılamadı. Program, grafik oturumu ve OpenGL 2.0 veya daha yenisini destekleyen bir ekran kartı sürücüsü gerektirir."
        }
        Text::PdnTooLarge => "Dosya bir oyun kaydı olamayacak kadar büyük.",
        Text::PdnEmpty => "Dosya boş: içinde oyun yok.",
        Text::PdnNotPdn => "Dosya PDN biçiminde bir oyun kaydına benzemiyor.",
        Text::PdnTooManyMoves => "Oyunda çok fazla hamle var.",
        Text::PdnWrongGameType => {
            "Bu, başka bir dama türünün oyunu (GameType “{value}”). \
             Yalnızca Rus daması (GameType 25) desteklenir."
        }
        Text::PdnBadFen => "FEN etiketindeki başlangıç konumu geçersiz: “{value}”.",
        Text::PdnBadTag => "Satır {line}: oyun başlığı bozuk (etiket kapatılmamış).",
        Text::PdnUnbalancedVariation => "Satır {line}: varyant parantezleri dengesiz.",
        Text::PdnVariationWithoutMove => "Satır {line}: varyant, ait olduğu hamleden önce başlıyor.",
        Text::PdnUnterminatedComment => "Satır {line}: yorum kapatılmamış.",
        Text::PdnBadMove => "Satır {line}: “{token}” — {problem}.",
        Text::MoveSyntax => "bu bir hamle değil",
        Text::MoveIllegal => "bu hamle bu konumda yapılamaz",
        Text::MoveAmbiguous => "taş alma gösterimi belirsiz",
        Text::MoveGameOver => "oyun bu noktada zaten bitmiş",
        Text::HumanName => "Oyuncu",
        Text::ComputerName => "Bilgisayar",
        Text::Keys => "Tuşlar",
        Text::KeysHelp => {
            "Sol ve sağ ok — hamleler arasında gezinme; \
             yukarı ve aşağı ok — varyantlar; \
             Home ve End — oyunun başı ve sonu; \
             Boşluk — buradan devam et; \
             Esc veya sağ fare düğmesi — seçimi iptal et; \
             F9 — hamle paneli; \
             Ctrl+N, Ctrl+O, Ctrl+S (macOS'ta Cmd) — yeni oyun, aç, kaydet"
        }
    }
}
