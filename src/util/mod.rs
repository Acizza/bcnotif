#[cfg(windows)]
pub mod windows;

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;

error_chain! {
    errors {
        EmptyFile(path: String) {
            description("empty file")
            display("file is empty: {}", path)
        }
    }
}

pub fn local_path(path: &str) -> Result<PathBuf> {
    let mut base = ::std::env::current_exe()
        .chain_err(|| format!("failed to get executable path"))?;

    base.pop();
    base.push(path);
    Ok(base)
}

fn get_path_str(path: &Path) -> &str {
    path.to_str().unwrap_or("<invalid path>")
}

/// Creates a file only if it doesn't already exist and returns whether it was created or not.
pub fn touch_file(path: &Path) -> Result<bool> {
    let exists = path.exists();

    if !exists {
        File::create(path)
            .chain_err(|| format!("failed to create file {}", get_path_str(path)))?;
    }

    Ok(exists)
}

pub fn verify_local_file(path: &str) -> Result<PathBuf> {
    let path = local_path(path)?;
    touch_file(&path)?;

    Ok(path)
}

pub fn read_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)
        .chain_err(|| format!("failed to open file {}", get_path_str(path)))?;

    let mut buffer = String::new();

    file.read_to_string(&mut buffer)
        .chain_err(|| format!("failed to read file {}", get_path_str(path)))?;

    if buffer.len() > 0 {
        Ok(buffer)
    } else {
        bail!(ErrorKind::EmptyFile(get_path_str(path).to_string()));
    }
}

pub fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    (1. - t) * v0 + t * v1
}

#[macro_export]
macro_rules! try_opt {
    ($value:expr) => {{
        match $value {
            Some(v) => v,
            None    => return None,
        }
    }};
}