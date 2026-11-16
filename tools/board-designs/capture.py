#!/usr/bin/env python3
"""Capture the native Rust viewer and package its framebuffer screenshots.

Requires the normal application build dependencies and ImageMagick. On a headless
Linux host run under xvfb-run. No image in this script is an input to board drawing.
"""
import argparse
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--frames", type=Path, default=Path("work/board-designs-native"))
    parser.add_argument("--output", type=Path, default=Path("docs/board-designs"))
    parser.add_argument("--reuse", action="store_true", help="Package existing native PPM captures")
    args = parser.parse_args()
    magick = shutil.which("magick")
    convert = [magick] if magick else [shutil.which("convert")]
    montage = [magick, "montage"] if magick else [shutil.which("montage")]
    if not convert[0] or not montage[0]:
        parser.error("ImageMagick is required to convert the screenshots and assemble comparison sheets")
    args.frames.mkdir(parents=True, exist_ok=True)
    args.output.mkdir(parents=True, exist_ok=True)
    if not args.reuse:
        subprocess.run([
            "cargo", "run", "--locked", "--example", "board_designs", "--",
            "--capture", str(args.frames.resolve()),
        ], check=True)

    groups = {"": [], "-with-pieces": [], "-detail": []}
    for number in range(1, 13):
        for suffix, files in groups.items():
            source = args.frames / f"{number:02}{suffix}.ppm"
            with source.open("rb") as stream:
                if stream.readline().strip() != b"P6":
                    raise ValueError(f"Not a native PPM capture: {source}")
                width, height = map(int, stream.readline().split())
                if width < 500 or height < 620 or stream.readline().strip() != b"255":
                    raise ValueError(f"Invalid capture dimensions: {source}")
                if len(stream.read()) != width * height * 3:
                    raise ValueError(f"Incomplete capture: {source}")
            destination = (args.frames if suffix == "-with-pieces" else args.output) / f"{number:02}{suffix}.png"
            subprocess.run(convert + [str(source), str(destination)], check=True)
            files.append(str(destination))
    for suffix, files in groups.items():
        name = "details.png" if suffix == "-detail" else f"overview{suffix}.png"
        subprocess.run(montage + files + [
            "-tile", "3x4", "-geometry", "560x620+8+8", "-background", "#d6c9b2",
            "-depth", "8", "-strip",
            str(args.output / name),
        ], check=True)
    print(f"Verified 36 native captures; saved 24 large PNGs and 3 comparison sheets to {args.output}")


if __name__ == "__main__":
    main()
