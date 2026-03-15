use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid repository: {0}")]
    InvalidRepo(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub version: u32,
    pub head: Option<usize>,
    pub commits: Vec<Commit>,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            version: 1,
            head: None,
            commits: Vec::new(),
        }
    }
}

pub struct Storage {
    root: PathBuf,
    artgit_dir: PathBuf,
    objects_dir: PathBuf,
    metadata_path: PathBuf,
}

impl Storage {
    pub fn init_layout(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let artgit_dir = root.join(".artgit");
        let objects_dir = artgit_dir.join("objects");
        let metadata_path = artgit_dir.join("metadata.json");

        if !artgit_dir.exists() {
            fs::create_dir_all(&objects_dir)?;
            let metadata = Metadata::default();
            let file = fs::File::create(&metadata_path)?;
            serde_json::to_writer_pretty(file, &metadata)?;
        } else {
            if !objects_dir.exists() {
                fs::create_dir_all(&objects_dir)?;
            }
            if !metadata_path.exists() {
                let metadata = Metadata::default();
                let file = fs::File::create(&metadata_path)?;
                serde_json::to_writer_pretty(file, &metadata)?;
            }
        }

        Ok(Self {
            root,
            artgit_dir,
            objects_dir,
            metadata_path,
        })
    }

    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let artgit_dir = root.join(".artgit");
        let objects_dir = artgit_dir.join("objects");
        let metadata_path = artgit_dir.join("metadata.json");

        if !artgit_dir.is_dir() {
            return Err(Error::InvalidRepo(format!(
                "No .artgit directory found at {}",
                artgit_dir.display()
            )));
        }

        Ok(Self {
            root,
            artgit_dir,
            objects_dir,
            metadata_path,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn load_metadata(&self) -> Result<Metadata> {
        let file = fs::File::open(&self.metadata_path)?;
        let metadata = serde_json::from_reader(file)?;
        Ok(metadata)
    }

    pub fn save_metadata(&self, metadata: &Metadata) -> Result<()> {
        let tmp_path = self.metadata_path.with_extension("json.tmp");
        {
            let file = fs::File::create(&tmp_path)?;
            serde_json::to_writer_pretty(file, metadata)?;
        }
        fs::rename(tmp_path, &self.metadata_path)?;
        Ok(())
    }

    fn object_path(&self, hash: &str) -> PathBuf {
        self.objects_dir.join(hash)
    }

    pub fn object_exists(&self, hash: &str) -> Result<bool> {
        Ok(self.object_path(hash).is_file())
    }

    pub fn write_object(&self, hash: &str, data: &[u8]) -> Result<()> {
        if self.object_exists(hash)? {
            return Ok(());
        }

        let compressed = zstd::encode_all(io::Cursor::new(data), 0)?;
        fs::write(self.object_path(hash), compressed)?;
        Ok(())
    }

    pub fn read_object(&self, hash: &str) -> Result<Vec<u8>> {
        let mut file = fs::File::open(self.object_path(hash))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        let decompressed = zstd::decode_all(io::Cursor::new(buf))?;
        Ok(decompressed)
    }
}
