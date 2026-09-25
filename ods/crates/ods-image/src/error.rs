//! Errors with context, printed the way VMS prints status codes.

use std::fmt;
use std::io;

/// The core's error, over image file I/O.
pub type OdsError = ods_core::Error<io::Error>;

#[derive(Debug)]
pub enum Kind {
    Ods(OdsError),
    /// I/O on a host file (not the image).
    Host(io::Error),
    /// Another process holds the image.
    Locked,
    /// A bad argument: path syntax, option value.
    Usage(String),
}

#[derive(Debug)]
pub struct Error {
    pub kind: Kind,
    /// What was being worked on: a file specification, a file ID, a path.
    pub context: String,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn usage(msg: impl Into<String>) -> Error {
        Error { kind: Kind::Usage(msg.into()), context: String::new() }
    }

    /// Adds context unless there is some already: the innermost is the most
    /// specific.
    pub fn at(mut self, ctx: impl fmt::Display) -> Error {
        if self.context.is_empty() {
            self.context = ctx.to_string();
        }
        self
    }

    /// The core error, if this is one.
    pub fn ods(&self) -> Option<&OdsError> {
        match &self.kind {
            Kind::Ods(e) => Some(e),
            _ => None,
        }
    }

    /// VMS style severity letter and status name.
    pub fn status(&self) -> (char, &'static str) {
        use ods_core::Error as E;
        match &self.kind {
            Kind::Host(_) => ('E', "HOSTIO"),
            Kind::Locked => ('E', "LOCKED"),
            Kind::Usage(_) => ('E', "USAGE"),
            Kind::Ods(e) => match e {
                E::Device(_) => ('F', "IOERR"),
                E::NoHomeBlock => ('F', "NOHOMEBLK"),
                E::Corrupt { .. } => ('F', "FILESTRUCT"),
                E::NotFound => ('E', "FNF"),
                E::DirNotFound => ('E', "DNF"),
                E::Stale(_) => ('E', "FILESEQCHK"),
                E::BadName(_) => ('E', "BADFILENAME"),
                E::Exists => ('E', "DUPFILENAME"),
                E::NotDirectory => ('E', "NOTDIR"),
                E::DirNotEmpty => ('E', "DIRNOTEMPTY"),
                E::NoVersion => ('E', "NOVERSION"),
                E::VersionOverflow => ('E', "VEROVF"),
                E::DeviceFull => ('E', "DEVICEFULL"),
                E::HeaderFull => ('E', "IDXFILEFULL"),
                E::ReadOnly => ('E', "WRITLCK"),
                E::Reserved => ('E', "NOPRIV"),
                E::BeyondEof => ('E', "ENDOFFILE"),
                E::Unsupported(_) => ('F', "UNSUPPORTED"),
                E::Invalid(_) => ('E', "INVARG"),
            },
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (sev, code) = self.status();
        write!(f, "%ODS-{sev}-{code}, ")?;
        match &self.kind {
            Kind::Ods(e) => write!(f, "{e}")?,
            Kind::Host(e) => write!(f, "{e}")?,
            Kind::Locked => f.write_str("image is in use by another process")?,
            Kind::Usage(m) => f.write_str(m)?,
        }
        if !self.context.is_empty() {
            write!(f, ": {}", self.context)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<OdsError> for Error {
    fn from(e: OdsError) -> Error {
        Error { kind: Kind::Ods(e), context: String::new() }
    }
}

impl From<io::Error> for Error {
    /// Unwraps our own errors that went through an `io::Write` or `Read`.
    fn from(e: io::Error) -> Error {
        if e.get_ref().is_some_and(|i| i.is::<Error>()) {
            if let Some(Ok(inner)) = e.into_inner().map(|b| b.downcast::<Error>()) {
                return *inner;
            }
            return Error::usage("error lost in transit");
        }
        Error { kind: Kind::Host(e), context: String::new() }
    }
}

/// Adds context to results.
pub trait Context<T> {
    fn at(self, ctx: impl fmt::Display) -> Result<T>;
}

impl<T, E: Into<Error>> Context<T> for std::result::Result<T, E> {
    fn at(self, ctx: impl fmt::Display) -> Result<T> {
        self.map_err(|e| e.into().at(ctx))
    }
}
