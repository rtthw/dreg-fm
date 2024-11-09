//! Dreg-based File Manager



use std::path::PathBuf;

use clap::Parser;
use dreg::prelude::*;
use widgets::Block;

mod widgets;



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
    fn update(&mut self, mut frame: Frame) {
        let area = frame.area;
        let (left_area, right_area) = area.hsplit_portion(0.2);
        let (middle_area, right_area) = right_area.hsplit_portion(0.5);

        Block::new(Style::new()).render(left_area, &mut frame.buffer);
        Block::new(Style::new()).render(middle_area, &mut frame.buffer);
        Block::new(Style::new()).render(right_area, &mut frame.buffer);
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

