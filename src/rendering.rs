use std::{
    fs::File,
    io::{self, BufReader, Read},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use image::{codecs::gif::GifDecoder, imageops::FilterType, AnimationDecoder, ImageBuffer, Rgb};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use std::io::Write;

use crate::{
    helpers::is_ffmpeg_available,
    helpers::rgba_to_rgb,
    helpers::{self, calculate_dimensions},
    models,
};

pub fn display_image(config: &models::Config) -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open(&config.path)?.to_rgb8();
    let terminal = models::Terminal::new();
    let (term_width, term_height) = terminal.size();

    let (new_width, new_height) = helpers::calculate_dimensions(
        img.width(),
        img.height(),
        config.width.unwrap_or(term_width),
        config.height.unwrap_or(term_height),
        config.by_height,
        config.stretch,
    );

    let resized = image::imageops::resize(&img, new_width, new_height, FilterType::Lanczos3);
    let adjusted = helpers::apply_adjustments(&resized, config.brightness, config.contrast);
    let ascii = helpers::to_ascii(&adjusted, config);

    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{}", ascii)?;
    stdout.flush()?;

    Ok(())
}

pub fn play_gif(
    config: &models::Config,
    stop: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(&config.path)?;
    let reader = BufReader::new(file);
    let decoder = GifDecoder::new(reader)?;
    let frames: Vec<_> = decoder.into_frames().collect::<Result<Vec<_>, _>>()?;

    if frames.is_empty() {
        return Err("GIF has no frames".into());
    }

    let mut terminal = models::Terminal::new();
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
            let resized =
                image::imageops::resize(&img, new_width, new_height, FilterType::Triangle);
            let adjusted = helpers::apply_adjustments(&resized, config.brightness, config.contrast);
            let ascii = helpers::to_ascii(&adjusted, config);

            let (num, denom) = frame.delay().numer_denom_ms();
            let ms = if denom == 0 {
                100
            } else {
                (num / denom).max(16)
            };
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

pub fn play_video(
    config: &models::Config,
    stop: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !is_ffmpeg_available() {
        return Err("ffmpeg not found. Please install ffmpeg to play videos.".into());
    }

    let mut terminal = models::Terminal::new();
    let (term_width, term_height) = terminal.size();

    let video_info = helpers::get_video_info(&config.path)?;
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

    let mut audio = models::AudioPlayer::new();
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

            let adjusted = helpers::apply_adjustments(&img, config.brightness, config.contrast);
            let ascii = helpers::to_ascii(&adjusted, config);

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
