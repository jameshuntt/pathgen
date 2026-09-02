use std::{fs, io, path::Path};

/// Shared operations for append-oriented record-stream file formats.
///
/// A record stream is not a nested document tree. It is a sequence of records
/// where appending and line/record boundaries matter. JSONL is the first
/// adapter that uses this pattern.
pub trait RecordStreamDocument {
    type Record: Clone;

    fn parse_records(s: &str) -> io::Result<Vec<Self::Record>>;
    fn records_to_string(records: &[Self::Record]) -> io::Result<String>;
    fn record_to_line(record: &Self::Record) -> io::Result<String>;

    fn load_records_file<P>(path: P) -> io::Result<Vec<Self::Record>>
    where
        P: AsRef<Path>,
    {
        let contents = fs::read_to_string(path)?;
        Self::parse_records(&contents)
    }

    fn write_records_file<P>(path: P, records: &[Self::Record]) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        fs::write(path, Self::records_to_string(records)?)
    }

    fn append_record_file<P>(path: P, record: &Self::Record) -> io::Result<()>
    where
        P: AsRef<Path>,
    {
        use std::io::Write;

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let mut line = Self::record_to_line(record)?;
        if !line.ends_with('\n') {
            line.push('\n');
        }
        file.write_all(line.as_bytes())
    }
}
