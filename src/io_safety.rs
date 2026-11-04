use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const MAX_INPUT_BYTES: usize = 1_048_576;

#[derive(Debug, thiserror::Error)]
pub enum SafetyError {
    #[error("refusing symlink: {0}")]
    Symlink(PathBuf),
    #[error("refusing non-regular file: {0}")]
    NotRegular(PathBuf),
    #[error("input exceeds 1 MiB: {0}")]
    TooLarge(PathBuf),
    #[error("output path already exists: {0}")]
    OutputExists(PathBuf),
    #[error("filesystem error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

fn path_error(path: &Path, source: io::Error) -> SafetyError {
    SafetyError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn check_regular_path(path: &Path) -> Result<(), SafetyError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| path_error(path, source))?;
    if metadata.file_type().is_symlink() {
        return Err(SafetyError::Symlink(path.to_path_buf()));
    }
    if !metadata.is_file() {
        return Err(SafetyError::NotRegular(path.to_path_buf()));
    }
    if metadata.len() > MAX_INPUT_BYTES as u64 {
        return Err(SafetyError::TooLarge(path.to_path_buf()));
    }
    Ok(())
}

pub fn read_bounded_file(path: &Path) -> Result<Vec<u8>, SafetyError> {
    check_regular_path(path)?;

    let file = File::open(path).map_err(|source| path_error(path, source))?;
    let path_metadata = fs::symlink_metadata(path).map_err(|source| path_error(path, source))?;
    if path_metadata.file_type().is_symlink() {
        return Err(SafetyError::Symlink(path.to_path_buf()));
    }
    if !path_metadata.is_file() {
        return Err(SafetyError::NotRegular(path.to_path_buf()));
    }
    let opened_metadata = file.metadata().map_err(|source| path_error(path, source))?;
    if !opened_metadata.is_file() {
        return Err(SafetyError::NotRegular(path.to_path_buf()));
    }
    if opened_metadata.len() > MAX_INPUT_BYTES as u64 {
        return Err(SafetyError::TooLarge(path.to_path_buf()));
    }

    let mut bytes = Vec::with_capacity(opened_metadata.len() as usize);
    file.take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| path_error(path, source))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(SafetyError::TooLarge(path.to_path_buf()));
    }
    Ok(bytes)
}

pub fn read_bounded_stdin<R: Read>(reader: R) -> Result<Vec<u8>, SafetyError> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| path_error(Path::new("stdin"), source))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(SafetyError::TooLarge(PathBuf::from("stdin")));
    }
    Ok(bytes)
}

pub fn check_output_available(path: &Path) -> Result<(), SafetyError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(SafetyError::Symlink(path.to_path_buf()))
        }
        Ok(metadata) if !metadata.is_file() => Err(SafetyError::NotRegular(path.to_path_buf())),
        Ok(_) => Err(SafetyError::OutputExists(path.to_path_buf())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(path_error(path, source)),
    }
}

pub fn write_output_atomic_no_clobber(path: &Path, contents: &[u8]) -> Result<(), SafetyError> {
    check_output_available(path)?;

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| {
            if source.kind() == io::ErrorKind::AlreadyExists {
                SafetyError::OutputExists(path.to_path_buf())
            } else {
                path_error(path, source)
            }
        })?;

    if let Err(source) = file.write_all(contents).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(path);
        return Err(path_error(path, source));
    }
    Ok(())
}
