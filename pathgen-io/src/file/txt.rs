use std::{fs, io, path::Path};

pub fn to_txt_file<P>(path: P, value: impl AsRef<str>) -> io::Result<()>
where
    P: AsRef<Path>,
{
    fs::write(path, value.as_ref())
}

pub fn from_txt_file<P>(path: P) -> io::Result<String>
where
    P: AsRef<Path>,
{
    fs::read_to_string(path)
}
