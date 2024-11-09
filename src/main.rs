//! Dreg-based File Manager



use std::path::PathBuf;

use clap::Parser;
use dreg::prelude::*;



fn main() -> Result<()> {
    let args = Cli::parse();

    if let Some(path) = args.path {
        std::env::set_current_dir(path)?;
    }

    CrosstermPlatform::new()?
        .run(FileManager {
            dir: std::env::current_dir()?,
            should_exit: false,
        })
}

/// Simple dreg-based file manager
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path for the working directory
    #[arg(short, long)]
    path: Option<PathBuf>,
}

pub struct FileManager {
    dir: PathBuf,
    should_exit: bool,
}

impl Program for FileManager {
    fn update(&mut self, frame: Frame) {
        let area = frame.area;
        frame.buffer.set_stringn(
            area.x,
            area.y,
            format!("{}", self.dir.display()),
            area.width as usize,
            Style::new(),
        );
    }

    fn on_input(&mut self, input: Input) {
        match input {
            Input::KeyDown(Scancode::Q) => {
                self.should_exit = true;
            }
            _ => {}
        }
    }

    fn on_platform_request(&mut self, _request: &str) -> Option<&str> { None }

    fn should_exit(&self) -> bool { self.should_exit }
}

