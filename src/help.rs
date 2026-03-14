pub fn print_help(program: &str) {
    eprintln!("pre v2.0");
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
