use std::env;

use crate::{help, models};

pub fn parse_args() -> Option<models::Config> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        help::print_help(&args[0]);
        return None;
    }

    let path = args[1].clone();

    let get_arg_value = |flag: &str| -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    };

    let charset = if args.contains(&"--simple".to_string()) {
        models::CharSet::Simple
    } else if args.contains(&"--blocks".to_string()) {
        models::CharSet::Blocks
    } else {
        models::CharSet::Detailed
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

    Some(models::Config {
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
