use std::{
    collections::HashSet,
    env::current_dir,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

pub fn list_subdirs<P>(directory: P, ignored: HashSet<&OsStr>) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    WalkDir::new(directory)
        .into_iter()
        .filter_entry(|entry| entry.file_type().is_dir() && !ignored.contains(entry.file_name()))
        .filter_map(|r| r.ok())
        // TODO: handle the error gracefully
        .map(|entry| entry.into_path().canonicalize().unwrap())
        .collect()
}

pub fn current_dir_string() -> String {
    String::from(current_dir().unwrap().to_str().unwrap())
}
