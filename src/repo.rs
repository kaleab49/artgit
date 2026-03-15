use crate::storage::{Commit, Error, FileEntry, Metadata, Result, Storage};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct Repo {
    root: PathBuf,
    storage: Storage,
    metadata: Metadata,
}

pub struct StatusReport {
    pub new: Vec<String>,
    pub modified: Vec<String>,
    pub unchanged: Vec<String>,
}

impl Repo {
    pub fn init(root: impl AsRef<Path>) -> Result<Self> {
        let storage = Storage::init_layout(&root)?;
        let metadata = storage.load_metadata()?;
        Ok(Self {
            root: storage.root().to_path_buf(),
            storage,
            metadata,
        })
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let storage = Storage::new(&root)?;
        let metadata = storage.load_metadata()?;
        Ok(Self {
            root: storage.root().to_path_buf(),
            storage,
            metadata,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn scan_files(&self) -> Result<Vec<FileEntry>> {
        let mut entries = Vec::new();

        for entry in WalkDir::new(&self.root).into_iter().filter_entry(|e| !is_ignored(e.path())) {
            let entry = entry.map_err(|e| Error::Io(io::Error::new(io::ErrorKind::Other, e)))?;
            let path = entry.path();
            if path.is_file() {
                let rel = path.strip_prefix(&self.root).unwrap().to_owned();
                let rel_str = rel.to_string_lossy().to_string();
                let (hash, size) = hash_file(path)?;
                entries.push(FileEntry {
                    path: rel_str,
                    hash,
                    size,
                });
            }
        }

        Ok(entries)
    }

    pub fn status(&self) -> Result<StatusReport> {
        let current = self.scan_files()?;

        let last_commit = self
            .metadata
            .head
            .and_then(|idx| self.metadata.commits.get(idx));

        let mut last_map: HashMap<&str, &FileEntry> = HashMap::new();
        if let Some(commit) = last_commit {
            for f in &commit.files {
                last_map.insert(&f.path, f);
            }
        }

        let mut new = Vec::new();
        let mut modified = Vec::new();
        let mut unchanged = Vec::new();

        for f in &current {
            match last_map.get(f.path.as_str()) {
                None => new.push(f.path.clone()),
                Some(prev) if prev.hash != f.hash => modified.push(f.path.clone()),
                Some(_) => unchanged.push(f.path.clone()),
            }
        }

        Ok(StatusReport {
            new,
            modified,
            unchanged,
        })
    }

    pub fn commit(&mut self, message: &str) -> Result<Commit> {
        let files = self.scan_files()?;

        let last_commit = self
            .metadata
            .head
            .and_then(|idx| self.metadata.commits.get(idx));

        if let Some(prev) = last_commit {
            if prev.files.len() == files.len()
                && prev
                    .files
                    .iter()
                    .zip(files.iter())
                    .all(|(a, b)| a.path == b.path && a.hash == b.hash)
            {
                return Err(Error::InvalidRepo(
                    "No changes detected since last commit".to_string(),
                ));
            }
        }

        for f in &files {
            if !self.storage.object_exists(&f.hash)? {
                let data = fs::read(self.root.join(&f.path))?;
                self.storage.write_object(&f.hash, &data)?;
            }
        }

        let timestamp = Utc::now();
        let commit_id = compute_commit_id(message, &timestamp.to_rfc3339(), &files)?;

        let commit = Commit {
            id: commit_id,
            message: message.to_string(),
            timestamp,
            files: files.clone(),
        };

        self.metadata.commits.push(commit.clone());
        self.metadata.head = Some(self.metadata.commits.len() - 1);
        self.storage.save_metadata(&self.metadata)?;

        Ok(commit)
    }

    pub fn log(&self) -> &[Commit] {
        &self.metadata.commits
    }
}

fn is_ignored(path: &Path) -> bool {
    if let Some(name) = path.file_name() {
        if name == ".artgit" {
            return true;
        }
    }
    false
}

fn hash_file(path: &Path) -> Result<(String, u64)> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    let mut size = 0u64;

    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
        size += read as u64;
    }

    let hash_bytes = hasher.finalize();
    let hash = hex::encode(hash_bytes);
    Ok((hash, size))
}

fn compute_commit_id(message: &str, timestamp: &str, files: &[FileEntry]) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(message.as_bytes());
    hasher.update(timestamp.as_bytes());

    let mut sorted = files.to_vec();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));

    for f in &sorted {
        hasher.update(f.path.as_bytes());
        hasher.update(f.hash.as_bytes());
    }

    let hash_bytes = hasher.finalize();
    Ok(hex::encode(hash_bytes))
}

