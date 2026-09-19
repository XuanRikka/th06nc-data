# Touhou Koumakyou: New Classic — Data Tools

**English | [中文](README.md) | [日本語](README.JA.md)**

This repository provides tools to unpack and repack the `Touhou Koumakyou: New Classic` package format, implemented through reverse engineering.

## Usage

### Install via cargo
```bash
cargo install --git https://github.com/XuanRikka/th06nc-data.git
```

### Download a prebuilt binary from Releases
Pick the build for your platform on the Releases page.

### Build from source
If you've already decided to do it this way, do you really need me to tell you how?

## Notes
 - Packing encryption is tightly coupled to the file name — unless you know what you are doing, do not rename files.
 - The package format has no concept of paths, so if two files in your packing directory share the same name, the behavior is undefined.
 - Packed file names must be pure ASCII.
 - Every archive always has a `ver0102.dat` entry at the end. If that file is present in your directory when repacking, pass `--no-ver0102` to avoid storing it twice.
 - An archive must contain a `ver0102.dat` at the end.

## Credits
 - Jun'ya Ota, the creator of the Touhou Project series, and every secondary creator of the Touhou series.
