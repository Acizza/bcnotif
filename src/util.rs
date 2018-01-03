use std::path::Path;
use std::fs::File;
use std::io::{self, Read};

pub fn read_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = String::new();

    file.read_to_string(&mut buffer)?;

    Ok(buffer)
}
