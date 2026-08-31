use std::fs::File;
use std::io::{self, Cursor, stderr, stdin, stdout};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::file::{FileId, FileIdFfi, FileOpenOptions};
use crate::{World, WorldError, WorldResult};

#[derive(Default)]
struct FileTable {
    open: Vec<Option<std::fs::File>>,
    free_list: Vec<FileId>,
}

impl FileTable {
    pub const fn new() -> Self {
        Self {
            open: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn open(&mut self, path: &Path, options: FileOpenOptions) -> WorldResult<FileId> {
        let mut open_options = std::fs::OpenOptions::new();
        open_options
            .read(options.contains(FileOpenOptions::READ))
            .write(options.contains(FileOpenOptions::WRITE))
            .create(options.contains(FileOpenOptions::CREATE))
            .create_new(options.contains(FileOpenOptions::CREATE_NEW))
            .append(options.contains(FileOpenOptions::APPEND))
            .truncate(options.contains(FileOpenOptions::TRUNCATE));

        let file = open_options.open(path)?;
        let file_id = match self.free_list.pop() {
            Some(id) => {
                self.open[id.to_ffi() as usize] = Some(file);
                id
            }
            None => {
                let id = FileId::from_ffi(self.open.len() as u64);
                self.open.push(Some(file));
                id
            }
        };
        Ok(file_id)
    }

    pub fn close(&mut self, handle: FileId) -> WorldResult<()> {
        let Some(file) = self.open[handle.to_ffi() as usize].take() else {
            return Err(WorldError::RecloseFile);
        };
        self.free_list.push(handle);

        use std::os::fd::IntoRawFd;
        let fd = file.into_raw_fd();

        // SAFETY:
        // - `fd` originates from `File::into_raw_fd`. Therefore, fd has sole ownership over this
        //   file descriptor.
        // - After this function, this file descriptor is gone and therefore it is closed exactly
        //   once.
        let close_result = unsafe { libc::close(fd) };

        // @NOTE: We don't retry closing ever. If it asks for a retry, just return an error.
        match close_result {
            0 => Ok(()),
            _ => Err(std::io::Error::last_os_error().into()),
        }
    }
    pub fn get(&mut self, handle: FileId) -> Option<&mut File> {
        self.open[handle.to_ffi() as usize].as_mut()
    }
}

/// Representation of the [`World`] that dispatches to standard operating system primitives.
#[derive(Default)]
pub struct StdWorld {
    file_table: FileTable,
    include_dirs: Vec<PathBuf>,
}

impl StdWorld {
    pub fn new() -> Self {
        Self {
            file_table: FileTable::new(),
            include_dirs: Vec::new(),
        }
    }

    pub fn push_include_dir(&mut self, include_dir: impl Into<PathBuf>) {
        self.include_dirs.push(include_dir.into());
    }
}

impl World for StdWorld {
    fn stdout(&mut self) -> Box<dyn io::Write> {
        Box::new(stdout().lock())
    }

    fn stderr(&mut self) -> Box<dyn io::Write> {
        Box::new(stderr().lock())
    }

    fn stdin(&mut self) -> Box<dyn io::Read> {
        Box::new(stdin().lock())
    }

    fn file_open(&mut self, path: &'_ Path, options: FileOpenOptions) -> WorldResult<FileId> {
        match self.file_table.open(path, options) {
            Ok(f) => Ok(f),
            Err(e) if !e.do_try_next_dir() || path.is_absolute() => return Err(e),
            Err(e) => {
                for p in &self.include_dirs {
                    match self.file_table.open(&p.join(path), options) {
                        Ok(f) => return Ok(f),
                        Err(_) if !e.do_try_next_dir() => return Err(e),
                        _ => {}
                    }
                }
                Err(e)
            }
        }
    }

    fn file_close(&mut self, handle: FileId) -> WorldResult<()> {
        self.file_table.close(handle)
    }

    fn file_write(&mut self, handle: FileId) -> Option<&mut dyn io::Write> {
        self.file_table.get(handle).map(|f| f as &mut dyn io::Write)
    }

    fn file_read(&mut self, handle: FileId) -> Option<&mut dyn io::Read> {
        self.file_table.get(handle).map(|f| f as &mut dyn io::Read)
    }

    fn read_to_string(&mut self, path: &'_ Path) -> WorldResult<String> {
        let result = std::fs::read_to_string(path).map_err(WorldError::from);
        match result {
            Ok(f) => Ok(f),
            Err(e) if !e.do_try_next_dir() || path.is_absolute() => return Err(e),
            Err(e) => {
                for p in &self.include_dirs {
                    let result = std::fs::read_to_string(&p.join(path)).map_err(WorldError::from);
                    match result {
                        Ok(f) => return Ok(f),
                        Err(_) if !e.do_try_next_dir() => return Err(e),
                        _ => {}
                    }
                }
                Err(e)
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct IoWriteCapture {
    inner: Arc<Mutex<Vec<u8>>>,
}
#[derive(Default, Clone)]
pub struct IoReadCapture {
    inner: Arc<Mutex<Cursor<Vec<u8>>>>,
}

impl IoWriteCapture {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn read_to_string(&mut self) -> Result<String, std::str::Utf8Error> {
        let buf = self.inner.lock().unwrap();
        std::str::from_utf8(buf.as_slice()).map(From::from)
    }
}

impl IoReadCapture {
    pub fn from_buffer(buffer: Vec<u8>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Cursor::new(buffer))),
        }
    }
}

impl io::Write for IoWriteCapture {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl io::Read for IoReadCapture {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.lock().unwrap().read(buf)
    }
}

/// Representation of the [`World`] that dispatches to standard operating system primitives, but
/// captures standard output, error and input streams.
#[derive(Default)]
pub struct StdWorldCaptured {
    inner: StdWorld,
    pub stdout: IoWriteCapture,
    pub stderr: IoWriteCapture,
    pub stdin: IoReadCapture,
}

impl World for StdWorldCaptured {
    fn stdout(&mut self) -> Box<dyn io::Write> {
        Box::new(self.stdout.clone())
    }

    fn stderr(&mut self) -> Box<dyn io::Write> {
        Box::new(self.stderr.clone())
    }

    fn stdin(&mut self) -> Box<dyn io::Read> {
        Box::new(self.stdin.clone())
    }

    fn file_open(&mut self, path: &'_ Path, options: FileOpenOptions) -> WorldResult<FileId> {
        self.inner.file_open(path, options)
    }

    fn file_close(&mut self, handle: FileId) -> WorldResult<()> {
        self.inner.file_close(handle)
    }

    fn file_write(&mut self, handle: FileId) -> Option<&mut dyn io::Write> {
        self.inner.file_write(handle)
    }

    fn file_read(&mut self, handle: FileId) -> Option<&mut dyn io::Read> {
        self.inner.file_read(handle)
    }
}
