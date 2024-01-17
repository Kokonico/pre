use image::{Rgb, ImageBuffer};
use std::env;
use image::imageops::FilterType;
use std::process;
use term_size;
use std::fmt::Write as FmtWrite;
use rayon::prelude::*;
use std::convert::TryInto;
use std::io::Write;

fn main() {
    // Get the command-line arguments.
    let args: Vec<String> = env::args().collect();

    // Check if the user provided a file path.
    if args.len() < 2 {
        println!("Please provide a file path.");
        process::exit(1);
    }

    // Load the image from the provided file path.
    let img = match image::open(&args[1]) {
        Ok(img) => img.to_rgb8(),
        Err(e) => {
            println!("Failed to open image: {}", e);
            process::exit(1);
        }
    };

    // Get the terminal size.
    let (terminal_width, terminal_height) = term_size::dimensions().unwrap();

    // Check if the --by_height flag is present.
    let by_height = args.contains(&"--height".to_string());

    // Check if the --stretch flag is present.
    let stretch = args.contains(&"--stretch".to_string());

    let (new_width, new_height) = if stretch {
        (terminal_width.try_into().unwrap(), terminal_height.try_into().unwrap())
    } else if by_height {
        let new_height = terminal_height.try_into().unwrap();
        let (width, height) = img.dimensions();
        let new_width = (new_height as f32 * width as f32 / height as f32).round() as u32;
        (new_width, new_height)
    } else {
        let new_width = terminal_width.try_into().unwrap();
        let (width, height) = img.dimensions();
        let new_height = (new_width as f32 * height as f32 / width as f32).round() as u32;
        (new_width, new_height)
    };

    // Resize the image.
    let resized_img = image::imageops::resize(&img, new_width, new_height, FilterType::Nearest);

    // Convert the resized image to ASCII art.
    let ascii_art = to_ascii_art(&resized_img);

    // Print the ASCII art.
    write!(std::io::stdout(), "{}", ascii_art).unwrap();
}

fn to_ascii_art(img: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> String {
    let ascii_chars = "$@B%8WM#*oahkbdpqwmZO0QCJYXzcvunxrjft/\\|()1{}[]?-_+~<>i!lI;:,\"^`'. ";
    let ascii_chars = ascii_chars.chars().rev().collect::<Vec<_>>();
    let height = img.height() as usize;

    // Process each row of the image in parallel.
    let ascii_art: String = (0..height).into_par_iter().map(|y| {
        let mut line = String::new();
        let mut current_color = None;

        for x in 0..img.width() as usize {
            let pixel = img.get_pixel(x as u32, y as u32);
            let luminance = (0.299*pixel[0] as f32 + 0.587*pixel[1] as f32 + 0.114*pixel[2] as f32) as usize;
            let ascii_index = luminance * ascii_chars.len() / 256;
            let color_code = format!("{};{};{}", pixel[0], pixel[1], pixel[2]);

            if Some(color_code.clone()) != current_color {
                write!(&mut line, "\x1b[38;2;{}m", color_code).unwrap();
                current_color = Some(color_code);
            }

            line.push(ascii_chars[ascii_index]);
        }

        write!(&mut line, "\x1b[0m").unwrap(); // Reset color at the end of each line.
        line
    }).collect::<Vec<_>>().join("\n");

    ascii_art
}