use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::{DateTime, Local, TimeZone};
use git2::{Oid, Repository, Signature, Sort};

use crate::weather::snapshot::WeatherSnapshot;

pub struct WitRepo {
    repo: Repository,
    #[allow(dead_code)]
    path: PathBuf,
}

impl WitRepo {
    pub fn init(path: &Path) -> anyhow::Result<Self> {
        let repo = Repository::init(path)?;

        // Create initial commit on an empty tree so we have HEAD
        let sig = Signature::now("wit", "wit@localhost")?;
        let tree_id = {
            let mut index = repo.index()?;
            index.write_tree()?
        };
        {
            let tree = repo.find_tree(tree_id)?;
            repo.commit(Some("HEAD"), &sig, &sig, "wit init", &tree, &[])?;
        }

        Ok(Self {
            repo,
            path: path.to_path_buf(),
        })
    }

    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let repo = Repository::open(path)?;
        Ok(Self {
            repo,
            path: path.to_path_buf(),
        })
    }

    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Stage all changed files and commit with given message
    pub fn commit_all(&self, message: &str) -> anyhow::Result<Oid> {
        let mut index = self.repo.index()?;
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .context("failed to stage files")?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let sig = Signature::now("wit", "wit@localhost")?;
        let parent = self.repo.head()?.peel_to_commit()?;

        let oid = self.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &[&parent],
        )?;

        Ok(oid)
    }

    /// Stage all changed files and commit with a backdated timestamp
    pub fn commit_all_at(&self, message: &str, epoch: i64) -> anyhow::Result<Oid> {
        let mut index = self.repo.index()?;
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .context("failed to stage files")?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        let sig = Signature::new("wit", "wit@localhost", &git2::Time::new(epoch, 0))?;
        let parent = self.repo.head()?.peel_to_commit()?;

        let oid = self.repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &[&parent],
        )?;

        Ok(oid)
    }

    /// Read a file from a specific commit
    pub fn read_file_at_commit(&self, oid: Oid, file_path: &str) -> anyhow::Result<String> {
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;
        let entry = tree
            .get_path(Path::new(file_path))
            .context(format!("file not found at commit: {}", file_path))?;
        let blob = self.repo.find_blob(entry.id())?;
        let content =
            std::str::from_utf8(blob.content()).context("file content is not valid UTF-8")?;
        Ok(content.to_string())
    }

    /// Walk commit history, optionally filtered by path
    pub fn walk_history(
        &self,
        path_filter: Option<&str>,
        max_count: usize,
    ) -> anyhow::Result<Vec<HistoryEntry>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        let mut entries = Vec::new();

        for oid_result in revwalk {
            if entries.len() >= max_count {
                break;
            }
            let oid = oid_result?;
            let commit = self.repo.find_commit(oid)?;

            // If path filter, check if this commit touches that path
            if let Some(filter_path) = path_filter {
                let dominated = self.commit_touches_path(&commit, filter_path);
                if !dominated {
                    continue;
                }
            }

            let time = commit.time();
            let dt = Local
                .timestamp_opt(time.seconds(), 0)
                .single()
                .unwrap_or_else(Local::now);

            entries.push(HistoryEntry {
                oid,
                message: commit.message().unwrap_or("").to_string(),
                timestamp: dt,
            });
        }

        Ok(entries)
    }

    fn commit_touches_path(&self, commit: &git2::Commit, path: &str) -> bool {
        let tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => return false,
        };

        // Check if the file exists in this commit's tree
        if tree.get_path(Path::new(path)).is_err() {
            return false;
        }

        // For initial commits or if no parent, consider it as touching
        if commit.parent_count() == 0 {
            return true;
        }

        // Compare with parent — if file changed, it touches the path
        if let Ok(parent) = commit.parent(0) {
            if let Ok(parent_tree) = parent.tree() {
                if let Ok(diff) = self.repo.diff_tree_to_tree(
                    Some(&parent_tree),
                    Some(&tree),
                    None,
                ) {
                    for delta in diff.deltas() {
                        let old_path = delta.old_file().path().unwrap_or(Path::new(""));
                        let new_path = delta.new_file().path().unwrap_or(Path::new(""));
                        if old_path.starts_with(path) || new_path.starts_with(path) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Find the most recent commit before a given datetime that touches a path
    pub fn find_commit_at_date(
        &self,
        target: DateTime<Local>,
        path_filter: Option<&str>,
    ) -> anyhow::Result<Option<Oid>> {
        let target_ts = target.timestamp();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        for oid_result in revwalk {
            let oid = oid_result?;
            let commit = self.repo.find_commit(oid)?;
            let commit_ts = commit.time().seconds();

            if commit_ts <= target_ts {
                if let Some(filter) = path_filter {
                    if self.commit_touches_path(&commit, filter) {
                        return Ok(Some(oid));
                    }
                } else {
                    return Ok(Some(oid));
                }
            }
        }

        Ok(None)
    }

    /// Get snapshot from N commits ago for a location
    #[allow(dead_code)]
    pub fn snapshot_at_offset(
        &self,
        location_slug: &str,
        offset: usize,
    ) -> anyhow::Result<Option<(Oid, WeatherSnapshot)>> {
        let file_path = format!("locations/{}/current.toml", location_slug);
        let history = self.walk_history(Some(&file_path), offset + 1)?;

        if let Some(entry) = history.get(offset) {
            let content = self.read_file_at_commit(entry.oid, &file_path)?;
            let snap = WeatherSnapshot::from_toml(&content)?;
            Ok(Some((entry.oid, snap)))
        } else {
            Ok(None)
        }
    }

    /// Get snapshot at a specific date for a location
    pub fn snapshot_at_date(
        &self,
        location_slug: &str,
        target: DateTime<Local>,
    ) -> anyhow::Result<Option<WeatherSnapshot>> {
        let file_path = format!("locations/{}/current.toml", location_slug);
        if let Some(oid) = self.find_commit_at_date(target, Some(&file_path))? {
            let content = self.read_file_at_commit(oid, &file_path)?;
            let snap = WeatherSnapshot::from_toml(&content)?;
            Ok(Some(snap))
        } else {
            Ok(None)
        }
    }
}

pub struct HistoryEntry {
    pub oid: Oid,
    pub message: String,
    pub timestamp: DateTime<Local>,
}
