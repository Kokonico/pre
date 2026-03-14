use image::codecs::gif::GifDecoder;
use image::imageops::FilterType;
use image::{AnimationDecoder, ImageBuffer, Rgb, RgbaImage};
use rayon::prelude::*;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const ASCII_DETAILED: &str =
    " .'`^\",:;Il!i><~+_-?][}{1)(|/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$";
const ASCII_SIMPLE: &str = " .:-=+*#%@";
const ASCII_BLOCKS: &str = " ░▒▓█";

const VIDEO_EXTENSIONS: &[&str] = &[
    ".mp4", ".mkv", ".avi", ".mov", ".webm", ".flv", ".wmv", ".m4v", ".mpeg", ".mpg", ".3gp",
];

struct Config {
    path: String,
    by_height: bool,
    stretch: bool,
    no_color: bool,
    invert: bool,
    loop_playback: bool,
    charset: CharSet,
    fps: f32,
    width: Option<u32>,
    height: Option<u32>,
    no_audio: bool,
    brightness: f32,
    contrast: f32,
}

#[derive(Clone, Copy)]
enum CharSet {
    Detailed,
    Simple,
    Blocks,
}

impl CharSet {
    fn chars(&self) -> &'static str {
        match self {
            CharSet::Detailed => ASCII_DETAILED,
            CharSet::Simple => ASCII_SIMPLE,
            CharSet::Blocks => ASCII_BLOCKS,
        }
    }
}

struct Terminal {
    cursor_hidden: bool,
    alternate_screen: bool,
}

impl Terminal {
    fn new() -> Self {
        Self {
            cursor_hidden: false,
            alternate_screen: false,
        }
    }

    fn enter_alternate_screen(&mut self) {
        if !self.alternate_screen {
            print!("\x1b[?1049h");
            let _ = io::stdout().flush();
            self.alternate_screen = true;
        }
    }

    fn leave_alternate_screen(&mut self) {
        if self.alternate_screen {
            print!("\x1b[?1049l");
            let _ = io::stdout().flush();
            self.alternate_screen = false;
        }
    }

    fn hide_cursor(&mut self) {
        if !self.cursor_hidden {
            print!("\x1b[?25l");
            let _ = io::stdout().flush();
            self.cursor_hidden = true;
        }
    }

    fn show_cursor(&mut self) {
        if self.cursor_hidden {
            print!("\x1b[?25h");
            let _ = io::stdout().flush();
            self.cursor_hidden = false;
        }
    }

    fn clear(&self) {
        print!("\x1b[2J\x1b[H");
        let _ = io::stdout().flush();
    }

    fn move_home(&self) {
        print!("\x1b[H");
        let _ = io::stdout().flush();
    }

    fn reset_colors(&self) {
        print!("\x1b[0m");
        let _ = io::stdout().flush();
    }

    fn size(&self) -> (u32, u32) {
        term_size::dimensions()
            .map(|(w, h)| (w as u32, h as u32))
            .unwrap_or((80, 24))
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.reset_colors();
        self.show_cursor();
        self.leave_alternate_screen();
    }
}

struct AudioPlayer {
    process: Option<Child>,
}

impl AudioPlayer {
    fn new() -> Self {
        Self { process: None }
    }

    fn play(&mut self, path: &str) -> bool {
        let result = Command::new("ffplay")
            .args(["-nodisp", "-autoexit", "-loglevel", "quiet", path])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match result {
            Ok(child) => {
                self.process = Some(child);
                true
            }
            Err(_) => false,
        }
    }

    fn stop(&mut self) {
        if let Some(ref mut child) = self.process {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.process = None;
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn main() {
    let config = match parse_args() {
        Some(c) => c,
        None => return,
    };

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);

    ctrlc::set_handler(move || {
        stop_clone.store(true, Ordering::SeqCst);
    })
    .expect("Failed to set Ctrl+C handler");

    let result = run(&config, &stop);

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn parse_args() -> Option<Config> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help(&args[0]);
        return None;
    }

    let path = args[1].clone();

    let get_arg_value = |flag: &str| -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    };

    let charset = if args.contains(&"--simple".to_string()) {
        CharSet::Simple
    } else if args.contains(&"--blocks".to_string()) {
        CharSet::Blocks
    } else {
        CharSet::Detailed
    };

    let fps = get_arg_value("--fps")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    let width = get_arg_value("--width").and_then(|s| s.parse().ok());

    let height_val = get_arg_value("--height").and_then(|s| s.parse().ok());

    let brightness = get_arg_value("--brightness")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.0);

    let contrast = get_arg_value("--contrast")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.0);

    Some(Config {
        path,
        by_height: args.contains(&"--fit-height".to_string()),
        stretch: args.contains(&"--stretch".to_string()),
        no_color: args.contains(&"--no-color".to_string()),
        invert: args.contains(&"--invert".to_string()),
        loop_playback: args.contains(&"--loop".to_string()),
        charset,
        fps,
        width,
        height: height_val,
        no_audio: args.contains(&"--no-audio".to_string()),
        brightness,
        contrast,
    })
}

fn print_help(program: &str) {
    eprintln!("ASCII Media Player v2.0");
    eprintln!();
    eprintln!("Usage: {} <file> [options]", program);
    eprintln!();
    eprintln!("Supported formats:");
    eprintln!("  Images: PNG, JPG, BMP, WEBP, etc.");
    eprintln!("  GIFs:   Animated GIF playback");
    eprintln!("  Videos: MP4, MKV, AVI, MOV, WEBM, etc. (requires ffmpeg)");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --fit-height       Scale by terminal height");
    eprintln!("  --stretch          Stretch to fill terminal");
    eprintln!("  --no-color         Disable colors");
    eprintln!("  --invert           Invert brightness");
    eprintln!("  --loop             Loop playback");
    eprintln!("  --simple           Use simple character set");
    eprintln!("  --blocks           Use block characters");
    eprintln!("  --fps <value>      Override frame rate");
    eprintln!("  --width <value>    Set output width");
    eprintln!("  --height <value>   Set output height");
    eprintln!("  --no-audio         Disable audio for videos");
    eprintln!("  --brightness <f>   Adjust brightness (default: 1.0)");
    eprintln!("  --contrast <f>     Adjust contrast (default: 1.0)");
    eprintln!("  --help, -h         Show this help");
    eprintln!();
    eprintln!("Controls:");
    eprintln!("  Ctrl+C             Stop playback");
}

fn run(config: &Config, stop: &Arc<AtomicBool>) -> Result<(), Box<dyn std::error::Error>> {
    let path_lower = config.path.to_lowercase();

    if path_lower.ends_with(".gif") {
        play_gif(config, stop)
    } else if VIDEO_EXTENSIONS.iter().any(|ext| path_lower.ends_with(ext)) {
        play_video(config, stop)
    } else {
        display_image(config)
    }
}

fn display_image(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open(&config.path)?.to_rgb8();
    let terminal = Terminal::new();
    let (term_width, term_height) = terminal.size();

    let (new_width, new_height) = calculate_dimensions(
        img.width(),
        img.height(),
        config.width.unwrap_or(term_width),
        config.height.unwrap_or(term_height),
        config.by_height,
        config.stretch,
    );

    let resized = image::imageops::resize(&img, new_width, new_height, FilterType::Lanczos3);
    let adjusted = apply_adjustments(&resized, config.brightness, config.contrast);
    let ascii = to_ascii(&adjusted, config);

    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{}", ascii)?;
    stdout.flush()?;

    Ok(())
}

fn play_gif(
    config: &Config,
    stop: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(&config.path)?;
    let reader = BufReader::new(file);
    let decoder = GifDecoder::new(reader)?;
    let frames: Vec<_> = decoder.into_frames().collect::<Result<Vec<_>, _>>()?;

    if frames.is_empty() {
        return Err("GIF has no frames".into());
    }

    let mut terminal = Terminal::new();
    let (term_width, term_height) = terminal.size();

    let first = frames[0].buffer();
    let (new_width, new_height) = calculate_dimensions(
        first.width(),
        first.height(),
        config.width.unwrap_or(term_width),
        config.height.unwrap_or(term_height),
        config.by_height,
        config.stretch,
    );

    let rendered: Vec<(String, Duration)> = frames
        .par_iter()
        .map(|frame| {
            let img = rgba_to_rgb(frame.buffer());
            let resized = image::imageops::resize(&img, new_width, new_height, FilterType::Triangle);
            let adjusted = apply_adjustments(&resized, config.brightness, config.contrast);
            let ascii = to_ascii(&adjusted, config);

            let (num, denom) = frame.delay().numer_denom_ms();
            let ms = if denom == 0 { 100 } else { (num / denom).max(16) };
            let duration = if config.fps > 0.0 {
                Duration::from_secs_f32(1.0 / config.fps)
            } else {
                Duration::from_millis(ms as u64)
            };

            (ascii, duration)
        })
        .collect();

    terminal.enter_alternate_screen();
    terminal.hide_cursor();
    terminal.clear();

    let mut stdout = io::stdout();

    'outer: loop {
        for (ascii, delay) in &rendered {
            if stop.load(Ordering::Relaxed) {
                break 'outer;
            }

            terminal.move_home();
            write!(stdout, "{}", ascii)?;
            stdout.flush()?;
            thread::sleep(*delay);
        }

        if !config.loop_playback {
            break;
        }
    }

    Ok(())
}

fn play_video(
    config: &Config,
    stop: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !is_ffmpeg_available() {
        return Err("ffmpeg not found. Please install ffmpeg to play videos.".into());
    }

    let mut terminal = Terminal::new();
    let (term_width, term_height) = terminal.size();

    let video_info = get_video_info(&config.path)?;
    let fps = if config.fps > 0.0 {
        config.fps
    } else {
        video_info.fps
    };

    let (new_width, new_height) = calculate_dimensions(
        video_info.width,
        video_info.height,
        config.width.unwrap_or(term_width),
        config.height.unwrap_or(term_height),
        config.by_height,
        config.stretch,
    );

    let mut audio = AudioPlayer::new();
    if !config.no_audio {
        audio.play(&config.path);
    }

    terminal.enter_alternate_screen();
    terminal.hide_cursor();
    terminal.clear();

    let frame_duration = Duration::from_secs_f32(1.0 / fps);
    let mut stdout = io::stdout();

    'outer: loop {
        let mut ffmpeg = Command::new("ffmpeg")
            .args([
                "-i",
                &config.path,
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-s",
                &format!("{}x{}", new_width, new_height),
                "-r",
                &fps.to_string(),
                "-loglevel",
                "quiet",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut reader = BufReader::new(ffmpeg.stdout.take().unwrap());
        let frame_size = (new_width * new_height * 3) as usize;
        let mut buffer = vec![0u8; frame_size];

        loop {
            if stop.load(Ordering::Relaxed) {
                let _ = ffmpeg.kill();
                break 'outer;
            }

            let start = Instant::now();

            match reader.read_exact(&mut buffer) {
                Ok(_) => {}
                Err(_) => break,
            }

            let img: ImageBuffer<Rgb<u8>, _> =
                ImageBuffer::from_raw(new_width, new_height, buffer.clone())
                    .ok_or("Failed to create image buffer")?;

            let adjusted = apply_adjustments(&img, config.brightness, config.contrast);
            let ascii = to_ascii(&adjusted, config);

            terminal.move_home();
            write!(stdout, "{}", ascii)?;
            stdout.flush()?;

            let elapsed = start.elapsed();
            if elapsed < frame_duration {
                thread::sleep(frame_duration - elapsed);
            }
        }

        let _ = ffmpeg.wait();

        if !config.loop_playback {
            break;
        }

        if !config.no_audio {
            audio.stop();
            audio.play(&config.path);
        }
    }

    Ok(())
}

struct VideoInfo {
    width: u32,
    height: u32,
    fps: f32,
}

fn get_video_info(path: &str) -> Result<VideoInfo, Box<dyn std::error::Error>> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate",
            "-of",
            "csv=p=0",
            path,
        ])
        .output()?;

    let text = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = text.trim().split(',').collect();

    if parts.len() < 3 {
        return Err("Could not parse video info".into());
    }

    let width: u32 = parts[0].parse()?;
    let height: u32 = parts[1].parse()?;

    let fps_parts: Vec<&str> = parts[2].split('/').collect();
    let fps = if fps_parts.len() == 2 {
        let num: f32 = fps_parts[0].parse().unwrap_or(30.0);
        let den: f32 = fps_parts[1].parse().unwrap_or(1.0);
        if den > 0.0 { num / den } else { 30.0 }
    } else {
        parts[2].parse().unwrap_or(30.0)
    };

    Ok(VideoInfo { width, height, fps })
}

fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn calculate_dimensions(
    img_width: u32,
    img_height: u32,
    term_width: u32,
    term_height: u32,
    by_height: bool,
    stretch: bool,
) -> (u32, u32) {
    const CHAR_ASPECT: f32 = 2.0;

    let usable_height = term_height.saturating_sub(1).max(1);
    let usable_width = term_width.max(1);

    if stretch {
        return (usable_width, usable_height);
    }

    if img_width == 0 || img_height == 0 {
        return (usable_width, usable_height);
    }

    let img_aspect = img_width as f32 / img_height as f32;

    if by_height {
        let new_height = usable_height;
        let new_width = (new_height as f32 * img_aspect * CHAR_ASPECT).round() as u32;
        (new_width.min(usable_width).max(1), new_height)
    } else {
        let new_width = usable_width;
        let new_height = (new_width as f32 / img_aspect / CHAR_ASPECT).round() as u32;

        if new_height > usable_height {
            let new_height = usable_height;
            let new_width = (new_height as f32 * img_aspect * CHAR_ASPECT).round() as u32;
            (new_width.min(usable_width).max(1), new_height)
        } else {
            (new_width, new_height.max(1))
        }
    }
}

fn rgba_to_rgb(img: &RgbaImage) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    ImageBuffer::from_fn(img.width(), img.height(), |x, y| {
        let p = img.get_pixel(x, y);
        let a = p[3] as f32 / 255.0;
        Rgb([
            (p[0] as f32 * a) as u8,
            (p[1] as f32 * a) as u8,
            (p[2] as f32 * a) as u8,
        ])
    })
}

fn apply_adjustments(
    img: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    brightness: f32,
    contrast: f32,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    if (brightness - 1.0).abs() < 0.001 && (contrast - 1.0).abs() < 0.001 {
        return img.clone();
    }

    ImageBuffer::from_fn(img.width(), img.height(), |x, y| {
        let p = img.get_pixel(x, y);
        let adjust = |v: u8| -> u8 {
            let f = v as f32 / 255.0;
            let f = ((f - 0.5) * contrast + 0.5) * brightness;
            (f.clamp(0.0, 1.0) * 255.0) as u8
        };
        Rgb([adjust(p[0]), adjust(p[1]), adjust(p[2])])
    })
}

fn to_ascii(img: &ImageBuffer<Rgb<u8>, Vec<u8>>, config: &Config) -> String {
    let chars: Vec<char> = if config.invert {
        config.charset.chars().chars().rev().collect()
    } else {
        config.charset.chars().chars().collect()
    };

    let height = img.height() as usize;
    let width = img.width() as usize;
    let num_chars = chars.len();

    if num_chars == 0 || width == 0 || height == 0 {
        return String::new();
    }

    let lines: Vec<String> = (0..height)
        .into_par_iter()
        .map(|y| {
            let mut line = String::with_capacity(width * 20);
            let mut last: Option<(u8, u8, u8)> = None;

            for x in 0..width {
                let p = img.get_pixel(x as u32, y as u32);
                let (r, g, b) = (p[0], p[1], p[2]);

                let lum = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as usize;
                let idx = (lum * (num_chars - 1) / 255).min(num_chars - 1);
                let ch = chars[idx];

                if config.no_color {
                    line.push(ch);
                } else {
                    let color = (r, g, b);
                    if last != Some(color) {
                        line.push_str(&format!("\x1b[38;2;{};{};{}m", r, g, b));
                        last = Some(color);
                    }
                    line.push(ch);
                }
            }

            if !config.no_color {
                line.push_str("\x1b[0m");
            }
            line
        })
        .collect();

    lines.join("\n")
}