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
            // SAFETY: 5 is obviously more than 0.
            file_cache: lru::LruCache::new(std::num::NonZeroUsize::new(5).unwrap()),
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
    /// (panel_index, listing_index)
    cursor_pos: (usize, usize),
    file_cache: lru::LruCache<PathBuf, FileData>,
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
        self.render_view(view_area.inner(1, 1), &mut frame.buffer);
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
            let fg = match entry.ty {
                FileType::Text => Color::Blue,
                FileType::Directory => Color::Green,
                FileType::Symlink => Color::Yellow,
                FileType::Image => Color::Cyan,
                FileType::Unknown => Color::Red,
                FileType::Video => Color::Magenta,
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

    fn render_view(&mut self, area: Rect, buf: &mut Buffer) {
        let Some(current_file) = self.iter_dir().nth(self.cursor_pos.1).cloned() else { return; };
        let file_data = self.file_cache.get_or_insert_mut(current_file.path.clone(), || {
            FileData::from(&current_file)
        });

        match file_data {
            FileData::Directory(content) => {
                let iter = content.children.iter()
                    .filter(|e| {
                        if !self.show_hidden_files {
                            if e.file_name.to_str().is_some_and(|s| s.starts_with(".")) {
                                return false;
                            }
                        }

                        true
                    });
                for (row, entry) in area.rows().into_iter().zip(iter) {
                    let fg = match entry.ty {
                        FileType::Text => Color::Blue,
                        FileType::Directory => Color::Green,
                        FileType::Symlink => Color::Yellow,
                        FileType::Image => Color::Cyan,
                        FileType::Unknown => Color::Red,
                        FileType::Video => Color::Magenta,
                    };
                    buf.set_stringn(
                        row.x,
                        row.y,
                        entry.file_name.to_string_lossy(),
                        row.width as usize,
                        Style::new().fg(fg),
                    );
                }
            }
            FileData::Text(content) => {
                for (row, line) in area.rows().into_iter().zip(content.lines()) {
                    buf.set_stringn(
                        row.x,
                        row.y,
                        line,
                        row.width as usize,
                        Style::new(),
                    );
                }
            }
            FileData::Error(msg) => {
                let msg_w = std::cmp::max(area.width as usize, msg.len());
                let msg_area = area.inner_centered(area.width, 1);
                buf.set_stringn(
                    msg_area.x,
                    msg_area.y,
                    msg,
                    msg_w,
                    Style::new().bold().fg(Color::Red),
                );
            }
            FileData::Null => {
                let msg_area = area.inner_centered(15, 1);
                buf.set_stringn(msg_area.x, msg_area.y, "Nothing here...", 15, Style::new().dim());
            }
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

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub file_name: OsString,
    pub ty: FileType,
}

impl From<DirEntry> for Entry {
    fn from(value: DirEntry) -> Self {
        Self {
            path: value.path(),
            file_name: value.file_name(),
            ty: FileType::from(&value),
        }
    }
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        matches!(self.ty, FileType::Directory)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FileType {
    Unknown,
    Directory,
    Symlink,
    Image,
    Text,
    Video,
}

impl From<&DirEntry> for FileType {
    fn from(value: &DirEntry) -> Self {
        let path = value.path();
        if path.is_dir() {
            FileType::Directory
        } else if path.is_symlink() {
            FileType::Symlink
        } else if path.is_file() {
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            match ext {
                "" => {
                    FileType::Text
                }
                "md" => FileType::Text,
                "rs" => FileType::Text,
                "toml" => FileType::Text,
                "yaml" | "yml" => FileType::Text,
                _ => FileType::Unknown,
                // e => todo!("handle {e} files"),
            }
        } else {
            FileType::Unknown
        }
    }
}

pub enum FileData {
    Directory(DirContent),
    Text(String),
    Null,
    Error(String),
}

impl From<&Entry> for FileData {
    fn from(value: &Entry) -> Self {
        match value.ty {
            FileType::Directory => {
                match DirContent::new(&value.path) {
                    Ok(dir_content) => Self::Directory(dir_content),
                    Err(e) => Self::Error(e.to_string()),
                }
            }
            FileType::Text => {
                match std::fs::read_to_string(&value.path) {
                    Ok(string) => Self::Text(string),
                    Err(e) => Self::Error(e.to_string()),
                }
            }
            _ => Self::Null,
        }
    }
}
