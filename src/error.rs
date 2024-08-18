mod error {

    use std::fmt;

    use cargo_lock;
    use reqwest;

    /// Error type.
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Error {
        /// Errors from cargo_lock
        CargoLock(cargo_lock::Error),

        /// Errors related to crate download
        Download(reqwest::Error),

        /// Errors related to crate lockfile extraction
        Tar(io::ErrorKind),

        /// Missing lockfile in tarball
        MissingLockfile,

        /// Could not parse the crate specification
        Spec(String),
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Error::CargoLock(error) => error.fmt(f),
                Error::Download(error) => error.fmt(f),
                Error::Tar(error) => error.fmt(f),
                Error::Spec(err) => write!(f, "invalid crate specifier: {}", err),
                Error::MissingLockfile => write!(f, "crate missing Cargo.lock file"),
            }
        }
    }

    impl From<io::Error> for Error {
        fn from(err: io::Error) -> Self {
            Error::Tar(err.kind())
        }
    }

    impl From<cargo_lock::Error> for Error {
        fn from(err: cargo_lock::Error) -> Self {
            Error::CargoLock(err)
        }
    }

    impl From<reqwest::Error> for Error {
        fn from(err: reqwest::Error) -> Self {
            Error::Download(err)
        }
    }

    impl std::error::Error for Error {}
}
