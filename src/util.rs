use std::path::Path;
use std::fs::File;
use std::io::Read;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
    }
}

pub fn read_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = String::new();

    file.read_to_string(&mut buffer)?;

    Ok(buffer)
}