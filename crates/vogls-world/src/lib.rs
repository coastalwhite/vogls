use ::std::io;
use ::std::path::Path;

use self::file::{FileId, FileOpenOptions};

mod file;
#[cfg(feature = "std")]
pub mod std;

#[non_exhaustive]
pub enum WorldError {
    RecloseFile,
    Io(io::Error),
}

impl From<io::Error> for WorldError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

type WorldResult<T> = ::std::result::Result<T, WorldError>;

/// Dynamic container for the operating environment of Vogls.
///
/// This trait handles all interaction with the operating system which allows operations to be
/// shimmed. As such, the trait is useful when sandboxing or embedding into environments that don't
/// use the typical OS primitives.
///
/// The typical standard [`World`] is the [`StdWorld`][std::StdWorld], which dispatches all
/// operations to the normal OS primitives.
pub trait World {
    /// Get a handle to the standard output stream.
    fn stdout(&mut self) -> Box<dyn io::Write>;

    /// Get a handle to the standard error stream.
    fn stderr(&mut self) -> Box<dyn io::Write>;

    /// Get a handle to the standard input stream.
    fn stdin(&mut self) -> Box<dyn io::Read>;

    /// Open a file to `path` with the given `options`.
    ///
    /// - Multiple instances of the same file can be open under different file handles.
    /// - File handles can be reused after they are closed.
    fn file_open(&mut self, path: &'_ Path, options: FileOpenOptions) -> WorldResult<FileId>;

    /// Close a file from a file handle.
    ///
    /// If closed on a file handle that is already closed and not reused, it will give a
    /// `RecloseFile` error.
    fn file_close(&mut self, handle: FileId) -> WorldResult<()>;

    /// Get a handle to write into a file.
    ///
    /// If the file handle has since been closed and not reused, it returns `None`.
    fn file_write<'a>(&'a mut self, handle: FileId) -> Option<&'a mut dyn io::Write>;

    /// Get a handle to read from a file.
    ///
    /// If the file handle has since been closed and not reused, it returns `None`.
    fn file_read<'a>(&'a mut self, handle: FileId) -> Option<&'a mut dyn io::Read>;

    /// Convenience method to read the file at `path` into a `String`.
    fn read_to_string(&mut self, path: &'_ Path) -> WorldResult<String> {
        let file_id = self.file_open(path, FileOpenOptions::READ)?;
        let reader = self.file_read(file_id).expect("Not closed before");
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        Ok(buf)
    }
}

/// A stub environment that never performs any IO or operating system calls.
///
/// - File opens always fail.
/// - Standard output and error writes succeed, but are directed no where.
/// - Standard input is empty.
pub struct NeverWorld;
struct IoNever;

impl io::Read for IoNever {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}
impl io::Write for IoNever {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl World for NeverWorld {
    fn stdout(&mut self) -> Box<dyn io::Write> {
        Box::new(IoNever)
    }

    fn stderr(&mut self) -> Box<dyn io::Write> {
        Box::new(IoNever)
    }

    fn stdin(&mut self) -> Box<dyn io::Read> {
        Box::new(IoNever)
    }

    fn file_open(&mut self, _path: &'_ Path, _options: FileOpenOptions) -> WorldResult<FileId> {
        Err(WorldError::Io(io::Error::from(io::ErrorKind::NotFound)))
    }

    fn file_close(&mut self, _handle: FileId) -> WorldResult<()> {
        Err(WorldError::RecloseFile)
    }

    fn file_write<'a>(&'a mut self, _handle: FileId) -> Option<&'a mut dyn io::Write> {
        None
    }

    fn file_read<'a>(&'a mut self, _handle: FileId) -> Option<&'a mut dyn io::Read> {
        None
    }
}

impl NeverWorld {
    pub const fn new() -> Self {
        Self
    }
}
