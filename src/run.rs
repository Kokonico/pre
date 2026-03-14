use std::sync::{atomic::AtomicBool, Arc};

use crate::{constants, models, rendering};

pub fn run(
    config: &models::Config,
    stop: &Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path_lower = config.path.to_lowercase();

    if path_lower.ends_with(".gif") {
        rendering::play_gif(config, stop)
    } else if constants::VIDEO_EXTENSIONS
        .iter()
        .any(|ext| path_lower.ends_with(ext))
    {
        rendering::play_video(config, stop)
    } else {
        rendering::display_image(config)
    }
}
