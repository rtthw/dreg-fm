//! Dreg-based File Manager



mod types;
mod widgets;

use std::{collections::HashSet, path::PathBuf};

use clap::Parser;
use dreg::prelude::*;
use types::*;
use widgets::*;



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
            marked_files: HashSet::new(),
            dialog: None,
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
    marked_files: HashSet<PathBuf>,
    dialog: Option<Dialog>,
}

impl Program for FileManager {
    fn update(&mut self, mut frame: Frame) {
        let full_area = frame.area;
        let (top_area, mut main_area) = full_area.vsplit_len(1);
        frame.buffer.set_style(top_area, Style::new().fg(Color::DarkGray).reversed());

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

        match self.cursor_pos.0 {
            1 => {
                Block::new(Style::new().dim()).render(main_area, &mut frame.buffer);
            }
            2 => {
                Block::new(Style::new().dim()).render(view_area, &mut frame.buffer);
            }
            _ => {}
        }

        self.render_middle(main_area.inner(1, 1), &mut frame.buffer);
        self.render_view(view_area.inner(1, 1), &mut frame.buffer);

        if let Some(dialog) = &self.dialog {
            match dialog {
                Dialog::ConfirmDelete => {
                    let dialog_area = full_area.inner(full_area.width / 4, full_area.height / 4);
                    Clear.render(dialog_area, &mut frame.buffer);
                    Block::new(Style::new()).render(dialog_area, &mut frame.buffer);
                    let inner_area = dialog_area.inner(1, 1);
                    let (dialog_top, dialog_bot) = inner_area.vsplit_portion(0.7);
                    let buttons_area = dialog_bot.inner_centered(9, 1);
                    let (yes_btn, _spacer, no_btn) = buttons_area.hsplit_even3();

                    let text = format!("Delete {} file(s)?", self.marked_files.len());
                    let text_width = std::cmp::min(text.len(), dialog_top.width as usize);

                    frame.buffer.set_stringn(
                        dialog_top.x,
                        dialog_top.y,
                        text,
                        text_width,
                        Style::new().bold(),
                    );

                    frame.buffer.set_stringn(yes_btn.x, yes_btn.y, "Y", 1, Style::new().underlined());
                    frame.buffer.set_stringn(yes_btn.x + 1, yes_btn.y, "es", 2, Style::new());
                    frame.buffer.set_stringn(no_btn.x, no_btn.y, "N", 1, Style::new().underlined());
                    frame.buffer.set_stringn(no_btn.x + 1, no_btn.y, "o", 1, Style::new());
                }
            }
        }
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
            Input::KeyDown(Scancode::ENTER) => {
                if self.cursor_pos.0 != 1 { return; }
                let Some(current_file) = self.iter_dir().nth(self.cursor_pos.1) else { return; };
                if current_file.is_dir() {
                    if let Ok(dir_content) = DirContent::new(&current_file.path) {
                        self.dir = dir_content;
                        self.cursor_pos.1 = 0;
                    }
                } else {
                    self.cursor_pos.1 = 2;
                }
            }
            Input::KeyDown(Scancode::PAGEUP) => {
                if self.cursor_pos.0 != 1 { return; }
                if let Some(parent_dir) = self.dir.path.parent() {
                    if let Ok(dir_content) = DirContent::new(parent_dir) {
                        self.dir = dir_content;
                        self.cursor_pos.1 = 0;
                    }
                }
            }
            Input::KeyDown(Scancode::APOSTROPHE) => {
                if self.cursor_pos.0 != 1 { return; }
                let Some(current_file) = self.iter_dir().nth(self.cursor_pos.1).cloned() else {
                    return;
                };
                if !self.marked_files.remove(&current_file.path) {
                    self.marked_files.insert(current_file.path);
                }
            }
            Input::KeyDown(Scancode::Y) => {
                match trash::delete_all(self.marked_files.drain()) {
                    Ok(_) => {
                        // TODO: Message log.
                    }
                    Err(e) => {
                        // TODO: Message log.
                        panic!("UNHANDLED ERROR: {e}");
                    }
                }
                self.dialog = None;
            }
            Input::KeyDown(Scancode::N) => {
                self.dialog = None;
            }
            Input::KeyDown(Scancode::D) => {
                if self.cursor_pos.0 != 1 { return; }
                if self.marked_files.is_empty() { return; }
                self.dialog = Some(Dialog::ConfirmDelete);
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
            let mut style = if index == self.cursor_pos.1 {
                Style::new().bold()
            } else {
                Style::new().dim()
            };
            if self.marked_files.contains(&entry.path) {
                style = style.underlined();
            }
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



pub enum Dialog {
    ConfirmDelete,
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
