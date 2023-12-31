use std::path::Path;
use std::{path::PathBuf, sync::Arc};
use std::io;

#[derive(Debug, Clone)]
pub struct WorkingDirectory {
    path: PathBuf,
}

impl WorkingDirectory {
    pub async fn open_or_create(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        tokio::fs::create_dir_all(&path).await?;
        Ok(WorkingDirectory { path })
    }

    pub async fn create_dir(&self, name: impl AsRef<Path>) -> io::Result<OwnedDir> {
        let dir = self.path.join(name.as_ref());
        tokio::fs::create_dir(&dir).await?;
        Ok(OwnedDir {
            path: dir,
        })
    }
}

#[derive(Debug)]
pub struct OwnedDir {
    path: PathBuf,
}

impl OwnedDir {
    pub fn create(path: PathBuf) -> io::Result<Self> {
        std::fs::create_dir(&path)?;
        Ok(OwnedDir { path })
    }

    pub fn into_shared(self) -> SharedDir {
        SharedDir(Arc::new(self))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, Clone)]
pub struct SharedDir(Arc<OwnedDir>);

impl SharedDir {
    pub fn path(&self) -> &Path {
        &self.0.path
    }

    pub fn claim_external_file(&self, name: &Path) -> OwnedFile {
        OwnedFile {
            path: self.0.path.join(name),
            _dir: Some(self.clone()),
        }
    }
}

#[derive(Debug)]
pub struct OwnedFile {
    path: PathBuf,
    _dir: Option<SharedDir>,
}

impl OwnedFile {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn into_shared(self) -> SharedFile {
        SharedFile(Arc::new(self))
    }
}

#[derive(Debug, Clone)]
pub struct SharedFile(Arc<OwnedFile>);

impl SharedFile {
    pub fn path(&self) -> &Path {
        self.0.path()
    }
}

impl Drop for OwnedDir {
    fn drop(&mut self) {
        match std::fs::remove_dir(&self.path) {
            Ok(()) => {}
            Err(e) => {
                log::warn!("error removing directory: {e:?}");
            }
        }
    }
}

impl Drop for OwnedFile {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(e) => {
                log::warn!("error removing file: {e:?}");
            }
        }
    }
}
