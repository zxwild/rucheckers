//! Indonesian.

#![allow(
    clippy::too_many_lines,
    reason = "the table has one arm for every interface text"
)]

use super::Text;

pub(super) const fn text(key: Text) -> &'static str {
    match key {
        Text::AppTitle => "Dam Rusia",
        Text::NewGame => "Permainan baru",
        Text::Open => "Buka…",
        Text::Save => "Simpan…",
        Text::Settings => "Pengaturan",
        Text::Close => "Tutup",
        Text::Cancel => "Batal",
        Text::Start => "Mulai",
        Text::Ok => "OK",
        Text::YourColor => "Warna Anda",
        Text::White => "Putih",
        Text::Black => "Hitam",
        Text::Level => "Tingkat",
        Text::LevelBeginner => "pemula",
        Text::LevelWeak => "lemah",
        Text::LevelAmateur => "amatir",
        Text::LevelSkilled => "berpengalaman",
        Text::LevelStrong => "kuat",
        Text::LevelMaximum => "maksimum",
        Text::Language => "Bahasa",
        Text::Sound => "Suara",
        Text::SoundOn => "Suara aktif",
        Text::SoundOff => "Suara mati",
        Text::MovesPanel => "Panel langkah",
        Text::Effects => "Efek khusus",
        Text::EffectsEnabled => "Tampilkan efek khusus",
        Text::EffectLightning => "Petir di belakang bidak saat memakan beberapa bidak",
        Text::EffectSparks => "Percikan dari bidak yang dimakan",
        Text::EffectFlashes => "Kilatan dan pendaran",
        Text::ShowPanel => "Tampilkan panel",
        Text::HidePanel => "Sembunyikan panel",
        Text::YourMove => "Giliran Anda",
        Text::MustCapture => "Giliran Anda: wajib memakan",
        Text::Thinking => "Komputer sedang berpikir…",
        Text::ComputerToMove => "Giliran komputer",
        Text::Review => "Tinjauan",
        Text::ContinueHere => "Lanjutkan dari sini (Spasi)",
        Text::ToStart => "Ke awal (Home)",
        Text::StepBack => "Mundur satu langkah (Panah kiri)",
        Text::StepForward => "Maju satu langkah (Panah kanan)",
        Text::ToEnd => "Ke akhir (End)",
        Text::Variations => "variasi:",
        Text::NoMovesYet => "Belum ada langkah",
        Text::YouPlayWhite => "Anda bermain Putih",
        Text::YouPlayBlack => "Anda bermain Hitam",
        Text::YouWin => "Anda menang",
        Text::YouLose => "Anda kalah",
        Text::Draw => "Seri",
        Text::WhiteNoPieces => "Putih tidak punya bidak lagi",
        Text::BlackNoPieces => "Hitam tidak punya bidak lagi",
        Text::WhiteNoMoves => "Putih tidak punya langkah yang sah",
        Text::BlackNoMoves => "Hitam tidak punya langkah yang sah",
        Text::DrawRepetition => "Posisi yang sama terjadi {count} kali",
        Text::DrawKingsVsKing => "Tiga raja atau lebih gagal memakan satu raja dalam {moves} langkah",
        Text::DrawKingsOnly => {
            "{moves} langkah hanya dimainkan oleh raja, tanpa memakan dan tanpa langkah bidak biasa"
        }
        Text::DrawBalance => {
            "Perimbangan kekuatan tidak berubah selama {moves} langkah \
             (kedua pihak memiliki raja, bidak di papan: {pieces})"
        }
        Text::DrawMainRoad => "Tiga bidak gagal memakan satu raja di diagonal utama dalam {moves} langkah",
        Text::OpenTitle => "Buka permainan",
        Text::SaveTitle => "Simpan permainan",
        Text::PdnFilter => "Permainan PDN",
        Text::ErrorTitle => "Kesalahan",
        Text::ReadFailed => "Berkas tidak dapat dibaca",
        Text::WriteFailed => "Berkas tidak dapat disimpan",
        Text::SettingsNotSaved => "Pengaturan tidak dapat disimpan",
        Text::StartFailed => {
            "Jendela tidak dapat dibuka. Program memerlukan sesi grafis dan driver grafis yang mendukung OpenGL 2.0 atau yang lebih baru."
        }
        Text::PdnTooLarge => "Berkas terlalu besar untuk sebuah catatan permainan.",
        Text::PdnEmpty => "Berkas kosong: tidak berisi permainan.",
        Text::PdnNotPdn => "Berkas tidak tampak seperti catatan permainan berformat PDN.",
        Text::PdnTooManyMoves => "Permainan berisi terlalu banyak langkah.",
        Text::PdnWrongGameType => {
            "Ini permainan dari varian dam lain (GameType “{value}”). \
             Hanya dam Rusia (GameType 25) yang didukung."
        }
        Text::PdnBadFen => "Posisi awal pada tag FEN tidak sah: “{value}”.",
        Text::PdnBadTag => "Baris {line}: kepala catatan permainan rusak (tag tidak ditutup).",
        Text::PdnUnbalancedVariation => "Baris {line}: tanda kurung variasi tidak seimbang.",
        Text::PdnVariationWithoutMove => "Baris {line}: variasi dimulai sebelum langkah yang dirujuknya.",
        Text::PdnUnterminatedComment => "Baris {line}: komentar tidak ditutup.",
        Text::PdnBadMove => "Baris {line}: “{token}” — {problem}.",
        Text::MoveSyntax => "ini bukan langkah",
        Text::MoveIllegal => "langkah ini tidak mungkin pada posisi ini",
        Text::MoveAmbiguous => "notasi memakannya ambigu",
        Text::MoveGameOver => "permainan sudah berakhir pada titik ini",
        Text::HumanName => "Pemain",
        Text::ComputerName => "Komputer",
        Text::Keys => "Tombol",
        Text::KeysHelp => {
            "Panah kiri dan kanan — menelusuri langkah; \
             atas dan bawah — variasi; \
             Home dan End — awal dan akhir permainan; \
             Spasi — lanjutkan dari sini; \
             Esc atau tombol kanan tetikus — batalkan pilihan; \
             F9 — panel langkah; \
             Ctrl+N, Ctrl+O, Ctrl+S (Cmd di macOS) — permainan baru, buka, simpan"
        }
    }
}
