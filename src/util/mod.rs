#[cfg(windows)]
pub mod windows;

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

#[macro_use]
pub mod error {
    use std::error::Error;

    #[derive(Debug)]
    pub struct DetailedError {
        pub err:  Box<Error>,
        pub func: &'static str,
        pub file: &'static str,
        pub line: u32,
    }

    #[macro_export]
    macro_rules! try_detailed {
        ($func:path, $($arg:expr),*) => {{
            match $func($($arg,)*) {
                Ok(v) => v,
                Err(err) => return Err(DetailedError {
                    err:  err.into(),
                    func: stringify!($func),
                    file: file!(),
                    line: line!(),
                }),
            }
        }};

        ($($token:tt)+) => {{
            match $($token)+ {
                Ok(v) => v,
                Err(err) => return Err(DetailedError {
                    err:  err.into(),
                    // Concatting each token removes unnecessary spacing
                    func: concat!($(stringify!($token),)+),
                    file: file!(),
                    line: line!(),
                }),
            }
        }};
    }

    /// Displays the provided error with a notification and by writing it to the terminal
    pub fn report(de: &DetailedError) {
        let msg = format!("[{}:{} {}]\nerror: ",
                    de.file,
                    de.line,
                    de.func);

        println!("{}{:?}", msg, de.err);

        match ::notification::create_error(&format!("{}{}", msg, de.err.description())) {
            Ok(_)    => (),
            Err(err) => println!("error creating error notification: {:?}", err),
        }
    }
}