# Pre
> preview images within the terminal

## dependencies

- [Rust](https://www.rust-lang.org/tools/install)
### crates:
- [image](https://crates.io/crates/image)
- [term_size](https://crates.io/crates/term_size)
- [rayon](https://crates.io/crates/rayon)

## Build/Install

```bash
cargo build --release
```
note: to use the command anywhere in the terminal, add the binary to your path, most of the time it will be in `target/release/`

## Usage

```bash
pre <image path>
```

## Flags

- `--height`: determines the images size based on the terminals height
- `--stretch`: stretches the image to fit the terminal
<br>
note: the image will be resized by width if no flags are provided.
