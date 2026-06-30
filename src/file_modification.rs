use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use napi_derive::napi;

/// Last commit that modified a file, with author/committer identity.
/// All times are `Date`s (UTC; timezone offset ignored).
#[napi(object)]
pub struct FileModification {
  /// 40-char lowercase hex OID of the last commit that modified the file.
  pub commit_id: String,
  /// Commit summary (first line). Undefined if absent or not valid UTF-8.
  pub summary: Option<String>,
  /// Author name. Undefined if not valid UTF-8.
  pub author_name: Option<String>,
  /// Author email. Undefined if not valid UTF-8.
  pub author_email: Option<String>,
  /// Author time, as a `Date`.
  pub author_time: DateTime<Utc>,
  /// Committer name. Undefined if not valid UTF-8.
  pub committer_name: Option<String>,
  /// Committer email. Undefined if not valid UTF-8.
  pub committer_email: Option<String>,
  /// Committer time, as a `Date`. Identical to `getFileLatestModifiedDate`.
  pub committer_time: DateTime<Utc>,
}

/// Convert git2 epoch seconds into a UTC `Date`. Errors (as a `git2::Error`, to
/// fit the surrounding history walk's `Result<_, git2::Error>`) only on the
/// practically unreachable out-of-range case.
pub(crate) fn time_to_date(seconds: i64) -> std::result::Result<DateTime<Utc>, git2::Error> {
  DateTime::from_timestamp(seconds, 0)
    .ok_or_else(|| git2::Error::from_str(&format!("Invalid commit timestamp: {seconds}")))
}

pub(crate) fn build_modification(
  commit: &git2::Commit,
) -> std::result::Result<FileModification, git2::Error> {
  let author = commit.author();
  let committer = commit.committer();
  // Mirrors the legacy value (repo.rs get_file_modified_date): commit.time(), NOT committer.when().
  let committer_time = time_to_date(commit.time().seconds())?;
  let author_time = time_to_date(author.when().seconds())?;
  Ok(FileModification {
    commit_id: commit.id().to_string(),
    summary: commit.summary().ok().flatten().map(|s| s.to_owned()),
    author_name: author.name().ok().map(|s| s.to_owned()),
    author_email: author.email().ok().map(|s| s.to_owned()),
    author_time,
    committer_name: committer.name().ok().map(|s| s.to_owned()),
    committer_email: committer.email().ok().map(|s| s.to_owned()),
    committer_time,
  })
}

/// Single-file walk: find the most recent commit that modified `filepath`.
/// Walks history from HEAD in time-topological order (newest first), diffing
/// each commit against its parent under a pathspec, and returns the first hit.
/// (Refactored from the legacy repo.rs get_file_modified_date; only the
/// returned value differs -- a struct instead of the bare i64.)
pub(crate) fn get_file_modification(
  repo: &git2::Repository,
  filepath: &str,
) -> std::result::Result<Option<FileModification>, git2::Error> {
  let mut diff_options = git2::DiffOptions::new();
  diff_options.disable_pathspec_match(false);
  diff_options.pathspec(filepath);
  let mut rev_walk = repo.revwalk()?;
  rev_walk.push_head()?;
  // Sort::TIME | Sort::TOPOLOGICAL: newest commits first (git-log order), so the
  // first commit whose diff touches the path is its latest modification.
  rev_walk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)?;
  let path = PathBuf::from(filepath);
  for oid in rev_walk.by_ref().filter_map(|oid| oid.ok()) {
    let Ok(commit) = repo.find_commit(oid) else {
      continue;
    };
    match commit.parent_count() {
      // commit with parent
      1 => {
        let Ok(tree) = commit.tree() else {
          continue;
        };
        if let Ok(parent) = commit.parent(0) {
          let Ok(parent_tree) = parent.tree() else {
            continue;
          };
          if let Ok(diff) =
            repo.diff_tree_to_tree(Some(&tree), Some(&parent_tree), Some(&mut diff_options))
            && diff.deltas().len() > 0
          {
            return Ok(Some(build_modification(&commit)?));
          }
        }
      }
      // root commit
      0 => {
        let Ok(tree) = commit.tree() else {
          continue;
        };
        if tree.get_path(&path).is_ok() {
          return Ok(Some(build_modification(&commit)?));
        }
      }
      // ignore merge commits
      _ => {}
    }
  }
  Ok(None)
}

/// Bulk walk: resolve the last commit that modified each of `filepaths` in a
/// SINGLE history walk. Inputs must be repo-root-relative FILE paths (not
/// directories): matching is exact-string against an `unresolved` set, NOT
/// glob/pathspec semantics. Every input path is a key; never-committed paths
/// map to `None`. Walks newest-first (time-topological), so the first commit
/// whose diff touches a path is that path's latest modification; early-exit
/// once `unresolved` empties.
pub(crate) fn get_files_modification(
  repo: &git2::Repository,
  filepaths: &[String],
) -> std::result::Result<HashMap<String, Option<FileModification>>, git2::Error> {
  let mut result: HashMap<String, Option<FileModification>> =
    filepaths.iter().map(|p| (p.clone(), None)).collect();
  let mut unresolved: HashSet<String> = filepaths.iter().cloned().collect();

  if unresolved.is_empty() {
    return Ok(result);
  }

  let mut diff_options = git2::DiffOptions::new();
  diff_options.disable_pathspec_match(false);
  for p in &unresolved {
    diff_options.pathspec(p);
  }

  let mut rev_walk = repo.revwalk()?;
  rev_walk.push_head()?;
  // Same newest-first (time-topological) order as the single-file walk.
  rev_walk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)?;

  for oid in rev_walk.by_ref().filter_map(|oid| oid.ok()) {
    if unresolved.is_empty() {
      break; // early-exit: nothing left to resolve
    }
    let commit = match repo.find_commit(oid) {
      Ok(c) => c,
      Err(_) => continue,
    };
    match commit.parent_count() {
      // commit with parent: diff (parent=old, commit=new) so added/modified
      // paths surface as new_file().path(); fall back to old_file() for deletes.
      1 => {
        let tree = match commit.tree() {
          Ok(t) => t,
          Err(_) => continue,
        };
        let parent = match commit.parent(0) {
          Ok(p) => p,
          Err(_) => continue,
        };
        let parent_tree = match parent.tree() {
          Ok(t) => t,
          Err(_) => continue,
        };
        if let Ok(diff) =
          repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut diff_options))
        {
          for delta in diff.deltas() {
            let path = delta
              .new_file()
              .path()
              .or_else(|| delta.old_file().path())
              .and_then(|p| p.to_str());
            if let Some(p) = path
              && unresolved.contains(p)
            {
              let key = p.to_owned();
              result.insert(key.clone(), Some(build_modification(&commit)?));
              unresolved.remove(&key);
            }
          }
        }
      }
      // root commit: probe each still-unresolved path in the tree
      0 => {
        if let Ok(tree) = commit.tree() {
          for p in unresolved.clone() {
            if tree.get_path(Path::new(&p)).is_ok() {
              result.insert(p.clone(), Some(build_modification(&commit)?));
              unresolved.remove(&p);
            }
          }
        }
      }
      // ignore merge commits
      _ => {}
    }
  }
  Ok(result)
}
