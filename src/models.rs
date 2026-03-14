use std::{
    io::{self, Write},
    process::{Child, Command, Stdio},
};

use crate::constants;

pub struct Config {
    pub path: String,
    pub by_height: bool,
    pub stretch: bool,
    pub no_color: bool,
    pub invert: bool,
    pub loop_playback: bool,
    pub charset: CharSet,
    pub fps: f32,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub no_audio: bool,
    pub brightness: f32,
    pub contrast: f32,
}

#[derive(Clone, Copy)]
pub enum CharSet {
    Detailed,
    Simple,
    Blocks,
}

impl CharSet {
    pub fn chars(&self) -> &'static str {
        match self {
            CharSet::Detailed => constants::ASCII_DETAILED,
            CharSet::Simple => constants::ASCII_SIMPLE,
            CharSet::Blocks => constants::ASCII_BLOCKS,
        }
    }
}

pub struct Terminal {
    pub cursor_hidden: bool,
    pub alternate_screen: bool,
}

impl Terminal {
    pub fn new() -> Self {
        Self {
            cursor_hidden: false,
            alternate_screen: false,
        }
    }

    pub fn enter_alternate_screen(&mut self) {
        if !self.alternate_screen {
            print!("\x1b[?1049h");
            let _ = io::stdout().flush();
            self.alternate_screen = true;
        }
    }

    pub fn leave_alternate_screen(&mut self) {
        if self.alternate_screen {
            print!("\x1b[?1049l");
            let _ = io::stdout().flush();
            self.alternate_screen = false;
        }
    }

    pub fn hide_cursor(&mut self) {
        if !self.cursor_hidden {
            print!("\x1b[?25l");
            let _ = io::stdout().flush();
            self.cursor_hidden = true;
        }
    }

    pub fn show_cursor(&mut self) {
        if self.cursor_hidden {
            print!("\x1b[?25h");
            let _ = io::stdout().flush();
            self.cursor_hidden = false;
        }
    }

    pub fn clear(&self) {
        print!("\x1b[2J\x1b[H");
        let _ = io::stdout().flush();
    }

    pub fn move_home(&self) {
        print!("\x1b[H");
        let _ = io::stdout().flush();
    }

    pub fn reset_colors(&self) {
        print!("\x1b[0m");
        let _ = io::stdout().flush();
    }

    pub fn size(&self) -> (u32, u32) {
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

pub struct AudioPlayer {
    pub process: Option<Child>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self { process: None }
    }

    pub fn play(&mut self, path: &str) -> bool {
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

    pub fn stop(&mut self) {
        if let Some(ref mut child) = self.process {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.process = None;
    }
}

pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub fps: f32,
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}
