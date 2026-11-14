#!/usr/bin/env python3
"""Rebuilds the CJK font subsets in assets/fonts from Noto Sans CJK.

Usage:
    tools/subset_cjk_fonts.py /path/to/NotoSansCJK-Regular.ttc

The fonts built into egui cover Latin and Cyrillic only. Each subset made here holds
exactly the characters that the Chinese, Japanese or Korean interface texts use, plus
the names of the three languages, so the files stay small (tens of kilobytes instead
of the 19 MB of the full family).

Run the script again after changing src/i18n/chinese.rs, japanese.rs or korean.rs:
until the fonts cover the texts, the test `gui::fonts::tests` fails.

Noto Sans CJK is distributed under the SIL Open Font License 1.1 (see
assets/fonts/OFL.txt); on Debian and Ubuntu the source file is in the package
fonts-noto-cjk. The script needs fontTools (`pip install fonttools`).
"""

import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTCollection

ROOT = Path(__file__).resolve().parent.parent
I18N = ROOT / "src" / "i18n"
OUT = ROOT / "assets" / "fonts"

# Output file, the face of the collection it is cut from, the texts it must cover.
SUBSETS = [
    ("NotoSansCJKsc-subset.otf", "Noto Sans CJK SC", "chinese.rs"),
    ("NotoSansCJKjp-subset.otf", "Noto Sans CJK JP", "japanese.rs"),
    ("NotoSansCJKkr-subset.otf", "Noto Sans CJK KR", "korean.rs"),
]

# Everything from the CJK Radicals block on: ideographs, kana, hangul, CJK punctuation
# and full-width forms. Characters below it come from the fonts built into egui.
FIRST_CJK = 0x2E80


def cjk_characters(path):
    return {ch for ch in path.read_text(encoding="utf-8") if ord(ch) >= FIRST_CJK}


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    collection = TTCollection(sys.argv[1])
    faces = {font["name"].getDebugName(4): font for font in collection.fonts}
    # The native names of the languages are listed in mod.rs and shown in every language.
    names = cjk_characters(I18N / "mod.rs")
    OUT.mkdir(parents=True, exist_ok=True)
    for file_name, face, source in SUBSETS:
        if face not in faces:
            sys.exit(f"{sys.argv[1]} has no face named {face!r}")
        characters = cjk_characters(I18N / source) | names
        options = subset.Options()
        options.layout_features = []  # egui does not shape text
        options.hinting = False
        options.notdef_outline = True
        options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14]  # keep copyright and licence
        subsetter = subset.Subsetter(options)
        subsetter.populate(unicodes=sorted(map(ord, characters)))
        font = faces[face]
        subsetter.subset(font)
        target = OUT / file_name
        font.save(target)
        print(f"{target.relative_to(ROOT)}: {len(characters)} characters, {target.stat().st_size} bytes")


if __name__ == "__main__":
    main()
