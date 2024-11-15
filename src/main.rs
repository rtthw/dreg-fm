//! Dreg-based File Manager



use std::{ffi::OsString, fs::DirEntry, path::PathBuf};

use clap::Parser;
use dreg::prelude::*;
use widgets::*;

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
            show_hidden_files: true,
            show_side_panel: false,
            input_handler: InputHandler::default(),
            cursor_pos: (1, 0),
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
    show_hidden_files: bool,
    show_side_panel: bool,
    input_handler: InputHandler,
    cursor_pos: (usize, usize),
}

impl Program for FileManager {
    fn update(&mut self, mut frame: Frame) {
        let mut main_area = frame.area;
        if self.show_side_panel {
            let left_block_style = if matches!(self.cursor_pos.0, 0) {
                Style::new()
            } else {
                Style::new().dim()
            };
            let (side_area, area) = main_area.hsplit_portion(0.2);
            main_area = area;
            Block::new(left_block_style).render(side_area, &mut frame.buffer);
        }
        let (main_area, view_area) = main_area.hsplit_portion(0.5);

        let mid_block_style = if matches!(self.cursor_pos.0, 1) {
            Style::new()
        } else {
            Style::new().dim()
        };
        let right_block_style = if matches!(self.cursor_pos.0, 2) {
            Style::new()
        } else {
            Style::new().dim()
        };
        Block::new(mid_block_style).render(main_area, &mut frame.buffer);
        Block::new(right_block_style).render(view_area, &mut frame.buffer);

        self.render_middle(main_area.inner(1, 1), &mut frame.buffer);
    }

    fn on_input(&mut self, input: Input) {
        match input {
            Input::KeyDown(Scancode::Q) => self.handle_command("exit"),
            Input::KeyDown(Scancode::H) => {
                if self.input_handler.alt {
                    self.handle_command("toggle_show_hidden_files");
                }
            }
            Input::KeyDown(Scancode::S) => {
                if self.input_handler.alt {
                    self.handle_command("toggle_show_side_panel");
                }
            }
            Input::KeyDown(Scancode::LEFT) => {
                if self.cursor_pos.0 == 1 {
                    if self.show_side_panel {
                        self.cursor_pos.0 = 0;
                    } else {
                        self.cursor_pos.0 = 2;
                    }
                } else if self.cursor_pos.0 == 0 {
                    self.cursor_pos.0 = 2;
                } else {
                    self.cursor_pos.0 = 1;
                }
            }
            Input::KeyDown(Scancode::RIGHT) => {
                if self.cursor_pos.0 == 1 {
                    self.cursor_pos.0 = 2;
                } else if self.cursor_pos.0 == 0 {
                    self.cursor_pos.0 = 1;
                } else {
                    if self.show_side_panel {
                        self.cursor_pos.0 = 0;
                    } else {
                        self.cursor_pos.0 = 1;
                    }
                }
            }
            Input::KeyDown(Scancode::UP) => {
                self.cursor_pos.1 = self.cursor_pos.1.saturating_sub(1);
            }
            Input::KeyDown(Scancode::DOWN) => {
                let e_count = self.iter_dir().count().saturating_sub(1);
                self.cursor_pos.1 = std::cmp::min(e_count, self.cursor_pos.1.saturating_add(1));
            }
            i => {
                self.input_handler.handle_input(i);
            }
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
            Command::ToggleShowHiddenFiles => {
                self.show_hidden_files = !self.show_hidden_files;
            }
            Command::ToggleShowSidePanel => {
                self.show_side_panel = !self.show_side_panel;
                if !self.show_side_panel && self.cursor_pos.0 == 0 {
                    self.cursor_pos.0 = 1;
                }
            }
        }
    }

    pub fn iter_dir(&self) -> impl Iterator<Item = &Entry> {
        self.dir.children.iter()
            .filter(|e| {
                if !self.show_hidden_files {
                    if e.file_name.to_str().is_some_and(|s| s.starts_with(".")) {
                        return false;
                    }
                }

                true
            })
    }
}

impl FileManager {
    fn render_middle(&mut self, area: Rect, buf: &mut Buffer) {
        // TODO: Scrolling.
        for (index, (row, entry)) in area.rows().into_iter().zip(self.iter_dir()).enumerate() {
            let fg = if entry.path.is_dir() {
                Color::Blue
            } else if entry.path.is_symlink() {
                Color::Yellow
            } else {
                Color::Gray
            };
            let style = if index == self.cursor_pos.1 {
                Style::new().bold()
            } else {
                Style::new().dim()
            };
            buf.set_stringn(
                row.x,
                row.y,
                entry.file_name.to_string_lossy(),
                row.width as usize,
                style.fg(fg),
            );
        }
    }
}



#[derive(Clone, Default)]
pub struct InputHandler {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl InputHandler {
    pub fn handle_input(&mut self, input: Input) {
        match input {
            Input::KeyDown(code) => match code {
                Scancode::L_CTRL | Scancode::R_CTRL => {
                    self.ctrl = true;
                }
                Scancode::L_ALT | Scancode::R_ALT => {
                    self.alt = true;
                }
                Scancode::L_SHIFT | Scancode::R_SHIFT => {
                    self.shift = true;
                }
                _ => {}
            }
            Input::KeyUp(code) => match code {
                Scancode::L_CTRL | Scancode::R_CTRL => {
                    self.ctrl = false;
                }
                Scancode::L_ALT | Scancode::R_ALT => {
                    self.alt = false;
                }
                Scancode::L_SHIFT | Scancode::R_SHIFT => {
                    self.shift = false;
                }
                _ => {}
            }
            _ => {}
        }
    }
}



#[derive(Clone, Copy)]
pub enum Command {
    Exit,
    ToggleShowHiddenFiles,
    ToggleShowSidePanel,
}

impl From<&'static str> for Command {
    fn from(value: &'static str) -> Self {
        match value {
            "exit" => Self::Exit,
            "toggle_show_hidden_files" => Self::ToggleShowHiddenFiles,
            "toggle_show_side_panel" => Self::ToggleShowSidePanel,
            // I don't believe static strings can be created dynamically, so this is fine.
            c => unreachable!("invalid command initializer: {c}"),
        }
    }
}



pub struct DirContent {
    pub path: PathBuf,
    pub children: Vec<Entry>,
}

impl DirContent {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let mut children: Vec<Entry> = std::fs::read_dir(&path)?
            .filter_map(|e| e.ok().and_then(|e| Some(Entry::from(e))))
            .collect();
        children.sort_by(|a, b| {
            if a.path.is_dir() == b.path.is_dir() {
                a.file_name.cmp(&b.file_name)
            } else if a.path.is_dir() {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        });

        Ok(Self {
            path,
            children,
        })
    }
}

pub struct Entry {
    pub path: PathBuf,
    pub file_name: OsString,
}

impl From<DirEntry> for Entry {
    fn from(value: DirEntry) -> Self {
        Self {
            path: value.path(),
            file_name: value.file_name(),
        }
    }
}
