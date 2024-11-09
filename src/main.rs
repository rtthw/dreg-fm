//! Dreg-based File Manager



use std::{fs::DirEntry, path::PathBuf};

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
            dir: DirContent::new(std::env::current_dir()?)?,
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
    dir: DirContent,
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

        self.render_middle(middle_area.inner(1, 1), &mut frame.buffer);
    }

    fn on_input(&mut self, input: Input) {
        match input {
            Input::KeyDown(Scancode::Q) => self.handle_command("exit"),
            _ => {}
        }
    }

    fn on_platform_request(&mut self, _request: &str) -> Option<&str> { None }

    fn should_exit(&self) -> bool { self.should_exit }
}

impl FileManager {
    pub fn handle_command(&mut self, command: impl Into<Command>) {
        match command.into() {
            Command::Exit => {
                self.should_exit = true;
            }
        }
    }
}

impl FileManager {
    fn render_middle(&mut self, area: Rect, buf: &mut Buffer) {
        // TODO: Scrolling.
        for (row, entry) in area.rows().into_iter().zip(self.dir.children.iter()) {
            buf.set_stringn(
                row.x,
                row.y,
                entry.file_name().to_string_lossy(),
                row.width as usize,
                Style::new().dim().fg(Color::Green),
            );
        }
    }
}



#[derive(Clone, Copy)]
pub enum Command {
    Exit,
}

impl From<&'static str> for Command {
    fn from(value: &'static str) -> Self {
        match value {
            "exit" => Self::Exit,
            c => unreachable!("invalid command initializer: {c}"),
        }
    }
}



pub struct DirContent {
    pub path: PathBuf,
    pub children: Vec<DirEntry>,
}

impl DirContent {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let children = std::fs::read_dir(&path)?
            .filter_map(|e| e.ok())
            .collect();

        Ok(Self {
            path,
            children,
        })
    }
}
