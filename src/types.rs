//! Type definitions



use dreg::prelude::Result;
use std::{ffi::OsString, fs::DirEntry, path::PathBuf};



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
                "asm" => FileType::Text,
                "lock" => FileType::Text,
                "md" => FileType::Text,
                "rs" => FileType::Text,
                "toml" => FileType::Text,
                "yaml" | "yml" => FileType::Text,
                "wat" => FileType::Text,
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
