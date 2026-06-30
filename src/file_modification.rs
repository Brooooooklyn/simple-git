use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Last commit that modified a file, with author/committer identity.
/// All times are ms since epoch (UTC; timezone offset ignored).
#[napi(object)]
pub struct FileModification {
  /// Committer time, ms since epoch. Identical to `getFileLatestModifiedDate`. Equals `committerTime`.
  pub timestamp: i64,
  /// 40-char lowercase hex OID of the last commit that modified the file.
  pub commit_id: String,
  /// Commit summary (first line). Undefined if absent or not valid UTF-8.
  pub summary: Option<String>,
  /// Author name. Undefined if not valid UTF-8.
  pub author_name: Option<String>,
  /// Author email. Undefined if not valid UTF-8.
  pub author_email: Option<String>,
  /// Author time, ms since epoch.
  pub author_time: i64,
  /// Committer name. Undefined if not valid UTF-8.
  pub committer_name: Option<String>,
  /// Committer email. Undefined if not valid UTF-8.
  pub committer_email: Option<String>,
  /// Committer time, ms since epoch. Equals `timestamp`.
  pub committer_time: i64,
}

pub(crate) fn build_modification(commit: &git2::Commit) -> FileModification {
  let author = commit.author();
  let committer = commit.committer();
  // Byte-identical to the legacy value (repo.rs get_file_modified_date): commit.time(), NOT committer.when().
  let committer_time = commit.time().seconds() * 1000;
  FileModification {
    timestamp: committer_time,
    commit_id: commit.id().to_string(),
    summary: commit.summary().ok().flatten().map(|s| s.to_owned()),
    author_name: author.name().ok().map(|s| s.to_owned()),
    author_email: author.email().ok().map(|s| s.to_owned()),
    author_time: author.when().seconds() * 1000,
    committer_name: committer.name().ok().map(|s| s.to_owned()),
    committer_email: committer.email().ok().map(|s| s.to_owned()),
    committer_time,
  }
}

/// Single-file walk. Mirrors the legacy repo.rs get_file_modified_date EXACTLY
/// (same revwalk, sort flag, pathspec, diff direction, merge-skip, root-commit
/// handling); only the returned value differs (struct instead of i64).
pub(crate) fn get_file_modification(
  repo: &git2::Repository,
  filepath: &str,
) -> std::result::Result<Option<FileModification>, git2::Error> {
  let mut diff_options = git2::DiffOptions::new();
  diff_options.disable_pathspec_match(false);
  diff_options.pathspec(filepath);
  let mut rev_walk = repo.revwalk()?;
  rev_walk.push_head()?;
  rev_walk.set_sorting(git2::Sort::TIME & git2::Sort::TOPOLOGICAL)?;
  let path = PathBuf::from(filepath);
  Ok(
    rev_walk
      .by_ref()
      .filter_map(|oid| oid.ok())
      .find_map(|oid| {
        let commit = repo.find_commit(oid).ok()?;
        match commit.parent_count() {
          // commit with parent
          1 => {
            let tree = commit.tree().ok()?;
            if let Ok(parent) = commit.parent(0) {
              let parent_tree = parent.tree().ok()?;
              if let Ok(diff) =
                repo.diff_tree_to_tree(Some(&tree), Some(&parent_tree), Some(&mut diff_options))
                && diff.deltas().len() > 0
              {
                return Some(build_modification(&commit));
              }
            }
          }
          // root commit
          0 => {
            let tree = commit.tree().ok()?;
            if tree.get_path(&path).is_ok() {
              return Some(build_modification(&commit));
            }
          }
          // ignore merge commits
          _ => {}
        };
        None
      }),
  )
}

/// Bulk walk: resolve the last commit that modified each of `filepaths` in a
/// SINGLE history walk. Every input path is a key; never-committed paths map to
/// `None`. Exact-string match against an `unresolved` set (NOT glob/pathspec
/// semantics); first (newest, since revwalk yields newest commits first under
/// the default order) hit wins; early-exit when `unresolved` empties.
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
  // default revwalk order (matches legacy single-file walk's sort flag)
  rev_walk.set_sorting(git2::Sort::TIME & git2::Sort::TOPOLOGICAL)?;

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
              result.insert(key.clone(), Some(build_modification(&commit)));
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
              result.insert(p.clone(), Some(build_modification(&commit)));
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
