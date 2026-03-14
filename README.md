# Pre
> preview images, gifs, and videos within the terminal

## Dependencies

- [Rust](https://www.rust-lang.org/tools/install)
- [FFmpeg](https://ffmpeg.org/download.html) (optional, for video playback)

### Crates:
- [image](https://crates.io/crates/image)
- [term_size](https://crates.io/crates/term_size)
- [rayon](https://crates.io/crates/rayon)
- [ctrlc](https://crates.io/crates/ctrlc)

## Build/Install

```bash
cargo build --release
```
note: to use the command anywhere in the terminal, add the binary to your path, most of the time it will be in `target/release/`

## Usage

```bash
pre <file path>
```

### Supported Formats
- **Images:** PNG, JPG, BMP, WEBP, etc.
- **GIFs:** Animated playback
- **Videos:** MP4, MKV, AVI, MOV, WEBM, etc. (requires ffmpeg)

## Flags

| Flag | Description |
|------|-------------|
| `--fit-height` | scale by terminal height |
| `--stretch` | stretch to fill terminal |
| `--no-color` | disable colors |
| `--invert` | invert brightness |
| `--loop` | loop playback |
| `--simple` | use simple character set |
| `--blocks` | use block characters (░▒▓█) |
| `--fps <n>` | override frame rate |
| `--width <n>` | set output width |
| `--height <n>` | set output height |
| `--no-audio` | disable audio for videos |
| `--brightness <f>` | adjust brightness (default: 1.0) |
| `--contrast <f>` | adjust contrast (default: 1.0) |
| `--help, -h` | show help |

note: the image will be scaled by width if no flags are provided.

## Controls

- `Ctrl+C` - stop playback
