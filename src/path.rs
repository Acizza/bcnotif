use directories::ProjectDirs;
use std::{io, fs};
use std::path::{Path, PathBuf};

lazy_static! {
    static ref PROJECT_DIRS: ProjectDirs = ProjectDirs::from("", "", env!("CARGO_PKG_NAME"));
}

fn get_path(dir: &Path, filename: &str) -> io::Result<PathBuf> {
    if !dir.exists() {
        fs::create_dir(dir)?;
    }

    let mut path = PathBuf::from(dir);
    path.push(filename);

    Ok(path)
}

pub fn get_config_file(name: &str) -> io::Result<PathBuf> {
    get_path(PROJECT_DIRS.config_dir(), name)
}

pub fn get_cache_file(name: &str) -> io::Result<PathBuf> {
    get_path(PROJECT_DIRS.cache_dir(), name)
}