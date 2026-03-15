use std::process::{Command, Stdio};

use image::{ImageBuffer, Rgb, RgbaImage};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::models;

pub fn calculate_dimensions(
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

pub fn apply_adjustments(
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

pub fn to_ascii(img: &ImageBuffer<Rgb<u8>, Vec<u8>>, config: &models::Config) -> String {
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

pub fn get_video_info(path: &str) -> Result<models::VideoInfo, Box<dyn std::error::Error>> {
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
        if den > 0.0 {
            num / den
        } else {
            30.0
        }
    } else {
        parts[2].parse().unwrap_or(30.0)
    };

    Ok(models::VideoInfo { width, height, fps })
}

pub fn is_ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn rgba_to_rgb(img: &RgbaImage) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
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
