use directories::ProjectDirs;
use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use std::{fs, io};

static PROJECT_DIRS: Lazy<ProjectDirs> = Lazy::new(|| {
    let dirs = ProjectDirs::from("", "", env!("CARGO_PKG_NAME"));

    match dirs {
        Some(dirs) => dirs,
        None => panic!("failed to get user directories"),
    }
});

fn get_path(dir: &Path, filename: &str) -> io::Result<PathBuf> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }

    let mut path = PathBuf::from(dir);
    path.push(filename);

    Ok(path)
}

pub fn get_config_file(name: &str) -> io::Result<PathBuf> {
    get_path(PROJECT_DIRS.config_dir(), name)
}

pub fn get_data_file(name: &str) -> io::Result<PathBuf> {
    get_path(PROJECT_DIRS.data_dir(), name)
}
