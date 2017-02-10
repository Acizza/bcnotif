#[macro_use] pub mod error;

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{self, Read, Error, ErrorKind};

pub fn local_path(path: &str) -> io::Result<PathBuf> {
    let mut base = ::std::env::current_exe()?;
    base.pop();
    base.push(path);
    Ok(base)
}

/// Creates a file only if it doesn't already exist and returns whether it was created or not.
pub fn touch_file(path: &Path) -> io::Result<bool> {
    let exists = path.exists();

    if !exists {
        File::create(path)?;
    }

    Ok(exists)
}

pub fn verify_local_file(path: &str) -> io::Result<PathBuf> {
    let path = local_path(path)?;
    touch_file(&path)?;

    Ok(path)
}

pub fn read_file(path: &Path) -> io::Result<String> {
    let mut file   = File::open(path)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;

    if buffer.len() > 0 {
        Ok(buffer)
    } else {
        let path = path.to_str().ok_or(Error::new(
                        ErrorKind::InvalidData,
                        "util::read_file(): malformed path"))?;
        
        Err(Error::new(ErrorKind::InvalidData, format!("util::read_file(): {} is empty", path)))
    }
}

pub fn lerp(v0: f32, v1: f32, t: f32) -> f32 {
    (1. - t) * v0 + t * v1
}