use std::io::{self, Error, ErrorKind};

/// Extension trait for ergonomic error-kind mapping from adapter codecs into
/// `std::io::Result`.
///
/// This intentionally lives in `pathgen-io`, not `pathgen-core`, because it is
/// useful to file/sql adapters but is not part of the route declaration kernel.
pub trait ResultExt<T> {
    fn invalid_data(self) -> io::Result<T>;
    fn not_found(self) -> io::Result<T>;
    fn permission_denied(self) -> io::Result<T>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn invalid_data(self) -> io::Result<T> {
        self.map_err(|e| Error::new(ErrorKind::InvalidData, e))
    }

    fn not_found(self) -> io::Result<T> {
        self.map_err(|e| Error::new(ErrorKind::NotFound, e))
    }

    fn permission_denied(self) -> io::Result<T> {
        self.map_err(|e| Error::new(ErrorKind::PermissionDenied, e))
    }
}
