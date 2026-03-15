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
    ignore_patterns: Vec<String>,
}

pub struct StatusReport {
    pub new: Vec<String>,
    pub modified: Vec<String>,
    pub unchanged: Vec<String>,
}

pub struct FileDiff {
    pub path: String,
    pub is_binary: bool,
    pub diff: Option<String>,
}

pub struct DiffReport {
    pub files: Vec<FileDiff>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Bundle {
    pub commits: Vec<Commit>,
    pub objects: std::collections::HashMap<String, Vec<u8>>,
}

pub struct BranchInfo {
    pub name: String,
    pub head_index: Option<usize>,
}

impl Repo {
    pub fn init(root: impl AsRef<Path>) -> Result<Self> {
        let storage = Storage::init_layout(&root)?;
        let metadata = storage.load_metadata()?;
        let root_path = storage.root().to_path_buf();
        let ignore_patterns = load_ignore_patterns(&root_path);
        Ok(Self {
            root: root_path,
            storage,
            metadata,
            ignore_patterns,
        })
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let storage = Storage::new(&root)?;
        let metadata = storage.load_metadata()?;
        let root_path = storage.root().to_path_buf();
        let ignore_patterns = load_ignore_patterns(&root_path);
        Ok(Self {
            root: root_path,
            storage,
            metadata,
            ignore_patterns,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn current_branch(&self) -> &str {
        &self.metadata.current_branch
    }

    fn scan_files(&self) -> Result<Vec<FileEntry>> {
        let mut entries = Vec::new();

        for entry in WalkDir::new(&self.root)
            .into_iter()
            .filter_entry(|e| !is_ignored(e.path(), &self.ignore_patterns))
        {
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
        // Update current branch pointer
        if let Some(head) = self.metadata.head {
            self.metadata
                .branches
                .insert(self.metadata.current_branch.clone(), head);
        }
        self.storage.save_metadata(&self.metadata)?;

        Ok(commit)
    }

    pub fn log(&self) -> &[Commit] {
        &self.metadata.commits
    }

    pub fn diff_working_vs_head(&self) -> Result<DiffReport> {
        let current = self.scan_files()?;
        let last_commit = self
            .metadata
            .head
            .and_then(|idx| self.metadata.commits.get(idx))
            .ok_or_else(|| Error::InvalidRepo("No commits to diff against".to_string()))?;

        let mut last_map: HashMap<&str, &FileEntry> = HashMap::new();
        for f in &last_commit.files {
            last_map.insert(&f.path, f);
        }

        let mut files = Vec::new();
        for cur in &current {
            if let Some(prev) = last_map.get(cur.path.as_str()) {
                if prev.hash != cur.hash {
                    files.push(self.diff_file(prev, cur)?);
                }
            } else {
                // New file: diff vs empty
                files.push(self.diff_file(
                    &FileEntry {
                        path: cur.path.clone(),
                        hash: String::new(),
                        size: 0,
                    },
                    cur,
                )?);
            }
        }

        Ok(DiffReport { files })
    }

    pub fn create_bundle(&self) -> Result<Bundle> {
        let mut objects = std::collections::HashMap::new();
        for commit in &self.metadata.commits {
            for f in &commit.files {
                if !objects.contains_key(&f.hash) {
                    let data = self.storage.read_object(&f.hash)?;
                    objects.insert(f.hash.clone(), data);
                }
            }
        }
        Ok(Bundle {
            commits: self.metadata.commits.clone(),
            objects,
        })
    }

    pub fn apply_bundle(&mut self, bundle: Bundle) -> Result<()> {
        // Write all objects
        for (hash, data) in &bundle.objects {
            self.storage.write_object(hash, data)?;
        }

        // Naive merge: append new commits whose ids are not present yet
        let existing_ids: std::collections::HashSet<String> =
            self.metadata.commits.iter().map(|c| c.id.clone()).collect();

        for commit in bundle.commits {
            if !existing_ids.contains(&commit.id) {
                self.metadata.commits.push(commit);
            }
        }

        self.storage.save_metadata(&self.metadata)?;
        Ok(())
    }

    fn diff_file(&self, prev: &FileEntry, cur: &FileEntry) -> Result<FileDiff> {
        let prev_bytes = if prev.hash.is_empty() {
            Vec::new()
        } else {
            self.storage.read_object(&prev.hash)?
        };
        let cur_bytes = fs::read(self.root.join(&cur.path))?;

        let prev_text = String::from_utf8(prev_bytes.clone());
        let cur_text = String::from_utf8(cur_bytes.clone());

        match (prev_text, cur_text) {
            (Ok(a), Ok(b)) => {
                let diff = unified_diff(&a, &b, &prev.path, &cur.path);
                Ok(FileDiff {
                    path: cur.path.clone(),
                    is_binary: false,
                    diff: Some(diff),
                })
            }
            _ => Ok(FileDiff {
                path: cur.path.clone(),
                is_binary: true,
                diff: None,
            }),
        }
    }

    pub fn list_branches(&self) -> Vec<BranchInfo> {
        let mut out = Vec::new();
        for (name, &idx) in &self.metadata.branches {
            out.push(BranchInfo {
                name: name.clone(),
                head_index: Some(idx),
            });
        }
        // Ensure current_branch is listed even if no entry yet
        if !self.metadata.branches.contains_key(&self.metadata.current_branch) {
            out.push(BranchInfo {
                name: self.metadata.current_branch.clone(),
                head_index: self.metadata.head,
            });
        }
        out
    }

    pub fn create_branch(&mut self, name: &str) -> Result<()> {
        if self.metadata.branches.contains_key(name) {
            return Err(Error::InvalidRepo(format!(
                "Branch {name} already exists"
            )));
        }
        let head = self
            .metadata
            .head
            .ok_or_else(|| Error::InvalidRepo("No commits yet to branch from".to_string()))?;
        self.metadata.branches.insert(name.to_string(), head);
        self.storage.save_metadata(&self.metadata)?;
        Ok(())
    }

    pub fn switch_branch(&mut self, name: &str) -> Result<()> {
        if !self.metadata.branches.contains_key(name) && name != "main" {
            return Err(Error::InvalidRepo(format!(
                "Branch {name} does not exist"
            )));
        }
        self.metadata.current_branch = name.to_string();
        if let Some(&idx) = self.metadata.branches.get(name) {
            self.metadata.head = Some(idx);
        }
        self.storage.save_metadata(&self.metadata)?;
        Ok(())
    }

    pub fn restore_file(&self, path: &str) -> Result<()> {
        let last_commit = self
            .metadata
            .head
            .and_then(|idx| self.metadata.commits.get(idx))
            .ok_or_else(|| Error::InvalidRepo("No commits to restore from".to_string()))?;

        let file = last_commit
            .files
            .iter()
            .find(|f| f.path == path)
            .ok_or_else(|| Error::InvalidRepo(format!("File {path} not found in last commit")))?;

        let data = self.storage.read_object(&file.hash)?;
        let out_path = self.root.join(&file.path);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(out_path, data)?;
        Ok(())
    }

    pub fn checkout_commit(&mut self, commit_id: &str, path: Option<&str>) -> Result<()> {
        let (idx, commit) = find_commit_by_prefix(&self.metadata.commits, commit_id)
            .ok_or_else(|| Error::InvalidRepo(format!("Commit {commit_id} not found")))?;

        match path {
            Some(p) => {
                let file = commit
                    .files
                    .iter()
                    .find(|f| f.path == p)
                    .ok_or_else(|| Error::InvalidRepo(format!("File {p} not found in commit")))?;
                let data = self.storage.read_object(&file.hash)?;
                let out_path = self.root.join(&file.path);
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(out_path, data)?;
            }
            None => {
                // Restore full tree: for now, only overwrite files from commit; do not delete extra working files.
                for file in &commit.files {
                    let data = self.storage.read_object(&file.hash)?;
                    let out_path = self.root.join(&file.path);
                    if let Some(parent) = out_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(out_path, data)?;
                }
            }
        }

        self.metadata.head = Some(idx);
        self.storage.save_metadata(&self.metadata)?;
        Ok(())
    }
}

fn is_ignored(path: &Path, patterns: &[String]) -> bool {
    if let Some(name) = path.file_name() {
        if name == ".artgit" {
            return true;
        }
    }

    // Very simple patterns: suffix match for extensions, prefix match for directories
    let rel = path.to_string_lossy();
    for pat in patterns {
        if pat.ends_with("/*") {
            let dir = &pat[..pat.len() - 1]; // keep trailing slash
            if rel.starts_with(dir) {
                return true;
            }
        } else if pat.starts_with("*.") {
            let ext = &pat[1..]; // ".ext"
            if rel.ends_with(ext) {
                return true;
            }
        } else if rel == *pat {
            return true;
        }
    }

    false
}

fn load_ignore_patterns(root: &Path) -> Vec<String> {
    let path = root.join(".artgitignore");
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
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

fn unified_diff(old: &str, new: &str, old_label: &str, new_label: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut out = String::new();
    out.push_str(&format!("--- {old_label}\n"));
    out.push_str(&format!("+++ {new_label}\n"));

    let mut i = 0usize;
    let mut j = 0usize;

    while i < old_lines.len() || j < new_lines.len() {
        if i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
            i += 1;
            j += 1;
        } else {
            if i < old_lines.len() {
                out.push_str(&format!("-{}\n", old_lines[i]));
                i += 1;
            }
            if j < new_lines.len() {
                out.push_str(&format!("+{}\n", new_lines[j]));
                j += 1;
            }
        }
    }

    out
}

fn find_commit_by_prefix<'a>(
    commits: &'a [Commit],
    prefix: &str,
) -> Option<(usize, &'a Commit)> {
    let mut found: Option<(usize, &Commit)> = None;
    for (idx, c) in commits.iter().enumerate() {
        if c.id.starts_with(prefix) {
            if found.is_some() {
                // ambiguous
                return None;
            }
            found = Some((idx, c));
        }
    }
    found
}

