use serde::{de::DeserializeOwned, Serialize};
use std::{fs, io, io::Result as IoResult, path::Path};

/// Binary CBOR file adapter.
///
/// This is intentionally a direct file codec, not a framed stream protocol.
/// Framed CBOR belongs in IPC/transport adapters; this module turns typed
/// Serde values into declared `file::cbor` payload bytes and back.
#[derive(Debug, Clone, Copy, Default)]
pub struct CborDocument;

impl CborDocument {
    pub fn to_vec<T: Serialize>(value: &T) -> IoResult<Vec<u8>> {
        to_cbor_vec(value)
    }

    pub fn from_slice<T: DeserializeOwned>(bytes: &[u8]) -> IoResult<T> {
        from_cbor_slice(bytes)
    }

    pub fn write_file<T, P>(path: P, value: &T) -> IoResult<()>
    where
        T: Serialize,
        P: AsRef<Path>,
    {
        to_cbor_file(path, value)
    }

    pub fn read_file<T, P>(path: P) -> IoResult<T>
    where
        T: DeserializeOwned,
        P: AsRef<Path>,
    {
        from_cbor_file(path)
    }
}

pub fn to_cbor_vec<T: Serialize>(value: &T) -> IoResult<Vec<u8>> {
    serde_cbor::to_vec(value).map_err(cbor_invalid_data)
}

pub fn from_cbor_slice<T: DeserializeOwned>(bytes: &[u8]) -> IoResult<T> {
    serde_cbor::from_slice(bytes).map_err(cbor_invalid_data)
}

pub fn to_cbor_file<T, P>(path: P, value: &T) -> IoResult<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    fs::write(path, to_cbor_vec(value)?)
}

pub fn from_cbor_file<T, P>(path: P) -> IoResult<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let bytes = fs::read(path)?;
    from_cbor_slice(&bytes)
}

pub fn to_cbor_value<T: Serialize>(value: &T) -> IoResult<serde_cbor::Value> {
    serde_cbor::value::to_value(value).map_err(cbor_invalid_data)
}

pub fn from_cbor_value<T: DeserializeOwned>(value: serde_cbor::Value) -> IoResult<T> {
    serde_cbor::value::from_value(value).map_err(cbor_invalid_data)
}

fn cbor_invalid_data(err: serde_cbor::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}
