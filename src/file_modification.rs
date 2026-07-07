use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// A single commit's identity/time metadata, reused for the commit that CREATED
/// a file (`FileModification::created`). Same 8 fields as `FileModification`'s
/// commit metadata. All times are `Date`s (UTC; timezone offset ignored).
#[napi(object)]
pub struct CommitInfo {
  /// 40-char lowercase hex OID of the commit that first added the file.
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
  /// Committer time, as a `Date`.
  pub committer_time: DateTime<Utc>,
}

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
  /// Committer time, as a `Date`. Identical to `getFileLastModifiedDate`.
  pub committer_time: DateTime<Utc>,
  /// The commit that FIRST added this file (its creation), resolved by an
  /// oldest-first ancestry walk over the EXACT repo-root-relative path -- SEPARATE
  /// from the newest-first modification walk above. `created` is a FILE's
  /// creation, so whether it resolves is decided by the queried path's ENTRY KIND
  /// AT HEAD: Undefined when a glob was passed, or when the path is a DIRECTORY (a
  /// tree entry) or a submodule (a gitlink/Commit entry) AT HEAD -- even if a FILE
  /// of that name existed earlier in history -- since no such entry is a file (the
  /// flat fields may still resolve via pathspec in those cases). A path that is a
  /// blob at HEAD, AND a DELETED file (absent at HEAD but a blob earlier in
  /// history), BOTH still report their ORIGINAL creation. (A path that no commit
  /// in history ever added yields no record at all -- the whole `FileModification`
  /// is `null`/absent -- not a present record with this field missing. A path
  /// present at HEAD that ONLY merge commits ever touched, on the other hand,
  /// yields a record built from its creating merge commit, with `commit_id ==
  /// created.commit_id`.) A delete-then-re-add returns the ORIGINAL add; merge
  /// commits are included; no rename-follow.
  pub created: Option<CommitInfo>,
}

/// Convert git2 epoch seconds into a UTC `Date`. Errors (as a `git2::Error`, to
/// fit the surrounding history walk's `Result<_, git2::Error>`) only on the
/// practically unreachable out-of-range case.
pub(crate) fn time_to_date(seconds: i64) -> std::result::Result<DateTime<Utc>, git2::Error> {
  DateTime::from_timestamp(seconds, 0)
    .ok_or_else(|| git2::Error::from_str(&format!("Invalid commit timestamp: {seconds}")))
}

/// Extract a commit's identity/time metadata once, shared by both the
/// modification record (`build_modification`) and the creation record
/// (`get_file_creation`). `committer_time` mirrors the legacy value
/// (repo.rs get_file_modified_date): `commit.time()`, NOT `committer.when()`.
pub(crate) fn build_commit_info(
  commit: &git2::Commit,
) -> std::result::Result<CommitInfo, git2::Error> {
  let author = commit.author();
  let committer = commit.committer();
  let committer_time = time_to_date(commit.time().seconds())?;
  let author_time = time_to_date(author.when().seconds())?;
  Ok(CommitInfo {
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

pub(crate) fn build_modification(
  commit: &git2::Commit,
) -> std::result::Result<FileModification, git2::Error> {
  // Reuse the shared 8-field extraction; `created` is merged in later by
  // get_file_modification_with_created (None until then).
  let info = build_commit_info(commit)?;
  Ok(FileModification {
    commit_id: info.commit_id,
    summary: info.summary,
    author_name: info.author_name,
    author_email: info.author_email,
    author_time: info.author_time,
    committer_name: info.committer_name,
    committer_email: info.committer_email,
    committer_time: info.committer_time,
    created: None,
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
  for oid in rev_walk.by_ref() {
    // Propagate revwalk iterator errors instead of silently skipping them.
    let oid = oid?;
    // A real object-read failure is an error, not a "no match" -- propagate it.
    let commit = repo.find_commit(oid)?;
    match commit.parent_count() {
      // commit with parent
      1 => {
        let tree = commit.tree()?;
        // parent_count() == 1 guarantees a parent exists; a read failure here is
        // a genuine error, so propagate rather than treating it as "no match".
        let parent = commit.parent(0)?;
        let parent_tree = parent.tree()?;
        let diff =
          repo.diff_tree_to_tree(Some(&tree), Some(&parent_tree), Some(&mut diff_options))?;
        // A successful diff with no delta means this commit didn't touch the
        // path -- that is a genuine "no match", so keep walking.
        if diff.deltas().len() > 0 {
          return Ok(Some(build_modification(&commit)?));
        }
      }
      // root commit
      0 => {
        let tree = commit.tree()?;
        // NotFound == "file absent from this root commit's tree" (no match); any
        // other lookup error is real and must propagate.
        match tree.get_path(&path) {
          Ok(_) => return Ok(Some(build_modification(&commit)?)),
          Err(e) if e.code() == git2::ErrorCode::NotFound => {}
          Err(e) => return Err(e),
        }
      }
      // ignore merge commits (documented semantic, not an error)
      _ => {}
    }
  }
  Ok(None)
}

/// Single-file creation walk: find the OLDEST commit that first added
/// `filepath` (its creation commit). Walks history from HEAD in
/// `Sort::TOPOLOGICAL | Sort::REVERSE` order (oldest-first ancestry; NO
/// `Sort::TIME`) and, for each commit, checks whether the tree contains the
/// EXACT repo-root-relative `path` via `tree.get_path` -- NOT pathspec/glob and
/// NOT parent-diffing, so merge commits are included on equal footing. Returns
/// the first (oldest) containing commit, early-exiting on that hit; a
/// delete-then-re-add therefore returns the ORIGINAL add. `Ok(None)` when no
/// commit in history ever contained the path. Mirrors the modification walk's
/// error-propagation: revwalk iterator + object-read failures propagate, and
/// only a genuine `NotFound` (path absent from a tree) is a non-error skip.
///
/// A HEAD-kind gate runs FIRST: `created` is a FILE's creation, so a path that
/// resolves to a TREE (directory) or GITLINK (submodule) AT HEAD yields `None`
/// even if a FILE of that name existed earlier in history. A path ABSENT at HEAD
/// (a deleted file) is NOT gated -- its original blob creation still resolves.
pub(crate) fn get_file_creation(
  repo: &git2::Repository,
  filepath: &str,
) -> std::result::Result<Option<CommitInfo>, git2::Error> {
  let path = PathBuf::from(filepath);
  // HEAD-kind gate: peel HEAD to its tree and inspect the queried path's entry.
  // `repo.head()` errors on an unborn/empty HEAD, consistent with `push_head()`.
  let head_tree = repo.head()?.peel_to_commit()?.tree()?;
  match head_tree.get_path(&path) {
    // A directory (tree) or submodule (gitlink) at HEAD is not a file -> no
    // creation, regardless of what history holds under that name.
    Ok(entry) if entry.kind() != Some(git2::ObjectType::Blob) => return Ok(None),
    // A real lookup error propagates; NotFound (path absent at HEAD = a deleted
    // file) is fine -- its original blob creation is still meaningful, so proceed.
    Err(e) if e.code() != git2::ErrorCode::NotFound => return Err(e),
    _ => {}
  }
  let mut rev_walk = repo.revwalk()?;
  rev_walk.push_head()?;
  // Oldest-first: the first commit whose tree contains the path is the one that
  // first added it, so we can early-exit on that hit.
  rev_walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::REVERSE)?;
  for oid in rev_walk.by_ref() {
    // Propagate revwalk iterator errors instead of silently skipping them.
    let oid = oid?;
    // A real object-read failure is an error, not a "no match" -- propagate it.
    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;
    // NotFound == path absent from this commit's tree (no match); any other
    // lookup error is real and must propagate.
    match tree.get_path(&path) {
      // Only an exact FILE (blob) is a creation. A directory (tree) or submodule
      // (gitlink/Commit) entry is NOT a file -> keep walking (no creation match).
      Ok(entry) if entry.kind() == Some(git2::ObjectType::Blob) => {
        return Ok(Some(build_commit_info(&commit)?));
      }
      Ok(_) => {}
      Err(e) if e.code() == git2::ErrorCode::NotFound => {}
      Err(e) => return Err(e),
    }
  }
  Ok(None)
}

/// Build a `FileModification` whose flat fields ARE the creating commit, used as
/// a fallback for a path present at HEAD that no non-merge commit ever modified
/// (introduced only by merge commits). Both the record and its `created` then
/// point at that creating commit.
fn file_modification_from_commit_info(c: CommitInfo) -> FileModification {
  FileModification {
    commit_id: c.commit_id.clone(),
    summary: c.summary.clone(),
    author_name: c.author_name.clone(),
    author_email: c.author_email.clone(),
    author_time: c.author_time,
    committer_name: c.committer_name.clone(),
    committer_email: c.committer_email.clone(),
    committer_time: c.committer_time,
    created: Some(c),
  }
}

/// Single-file modification lookup enriched with the file's `created` commit.
/// Runs the newest-first modification walk (`get_file_modification`); when that
/// yields a record (`Some`), the SEPARATE oldest-first creation walk
/// (`get_file_creation`) is merged into `FileModification::created`. When the
/// modification walk finds NOTHING (`None`) -- no NON-MERGE commit ever touched
/// the path -- the creation walk is the fallback: if it resolves (the file
/// exists at HEAD, introduced/last-touched only by MERGE commits, which the
/// modification walk skips), the whole record IS that creating (merge) commit,
/// so `commit_id == created.commit_id`. A path that no commit ever added stays
/// `None`. Shared by the sync method and the async task so both produce
/// identical results.
pub(crate) fn get_file_modification_with_created(
  repo: &git2::Repository,
  filepath: &str,
) -> std::result::Result<Option<FileModification>, git2::Error> {
  match get_file_modification(repo, filepath)? {
    Some(mut fm) => {
      fm.created = get_file_creation(repo, filepath)?;
      Ok(Some(fm))
    }
    // No non-merge commit modified the path. If the file still exists (only merge
    // commits touched it), report the creating commit as the whole record instead
    // of null. A path that never existed stays None.
    None => Ok(get_file_creation(repo, filepath)?.map(file_modification_from_commit_info)),
  }
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

  for oid in rev_walk.by_ref() {
    if unresolved.is_empty() {
      break; // early-exit: nothing left to resolve
    }
    // Propagate revwalk iterator errors instead of silently skipping them.
    let oid = oid?;
    // A real object-read failure is an error, not a "no match" -- propagate it.
    let commit = repo.find_commit(oid)?;
    match commit.parent_count() {
      // commit with parent: diff (parent=old, commit=new) so added/modified
      // paths surface as new_file().path(); fall back to old_file() for deletes.
      1 => {
        let tree = commit.tree()?;
        // parent_count() == 1 guarantees a parent exists; propagate a real read
        // failure rather than treating it as "no match".
        let parent = commit.parent(0)?;
        let parent_tree = parent.tree()?;
        let diff =
          repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut diff_options))?;
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
      // root commit: probe each still-unresolved path in the tree
      0 => {
        let tree = commit.tree()?;
        for p in unresolved.clone() {
          // NotFound == path absent from this root tree (no match); any other
          // lookup error is real and must propagate.
          match tree.get_path(Path::new(&p)) {
            Ok(_) => {
              result.insert(p.clone(), Some(build_modification(&commit)?));
              unresolved.remove(&p);
            }
            Err(e) if e.code() == git2::ErrorCode::NotFound => {}
            Err(e) => return Err(e),
          }
        }
      }
      // ignore merge commits
      _ => {}
    }
  }
  Ok(result)
}

/// Bulk creation walk: resolve the OLDEST commit that first added each of
/// `filepaths` in a SINGLE oldest-first ancestry walk. Mirrors the single-file
/// `get_file_creation` per-commit logic (EXACT `tree.get_path`, merges included,
/// first/oldest containing commit wins) but resolves many paths at once, reusing
/// the `unresolved`-set + early-exit structure of `get_files_modification`.
/// Inputs must be repo-root-relative FILE paths; matching is exact-string, NOT
/// glob/pathspec. Every input path is a key; a path never present in any tree
/// maps to `None`. Empty input early-returns before touching the revwalk.
///
/// A HEAD-kind gate runs FIRST (bulk form of `get_file_creation`'s): any path
/// that is a TREE (directory) or GITLINK (submodule) AT HEAD is dropped from
/// `unresolved` and stays `None` -- it is not a file, so it has no creation. A
/// path ABSENT at HEAD (a deleted file) or a blob at HEAD stays in `unresolved`.
pub(crate) fn get_files_creation(
  repo: &git2::Repository,
  filepaths: &[String],
) -> std::result::Result<HashMap<String, Option<CommitInfo>>, git2::Error> {
  let mut result: HashMap<String, Option<CommitInfo>> =
    filepaths.iter().map(|p| (p.clone(), None)).collect();
  let mut unresolved: HashSet<String> = filepaths.iter().cloned().collect();

  if unresolved.is_empty() {
    return Ok(result);
  }

  // HEAD-kind gate: peel HEAD once and drop any path that is a TREE (directory)
  // or GITLINK (submodule) AT HEAD -- such a path is not a file, so it has no
  // creation and stays `None`. A path ABSENT at HEAD (NotFound == a deleted file)
  // or a blob at HEAD stays in `unresolved` for the oldest-blob reverse walk. A
  // plain loop (not `retain`) so a non-`NotFound` lookup error can propagate.
  let head_tree = repo.head()?.peel_to_commit()?.tree()?;
  for p in unresolved.clone() {
    match head_tree.get_path(Path::new(&p)) {
      Ok(entry) if entry.kind() != Some(git2::ObjectType::Blob) => {
        unresolved.remove(&p);
      }
      Ok(_) => {}
      Err(e) if e.code() == git2::ErrorCode::NotFound => {}
      Err(e) => return Err(e),
    }
  }
  if unresolved.is_empty() {
    return Ok(result);
  }

  let mut rev_walk = repo.revwalk()?;
  rev_walk.push_head()?;
  // Oldest-first ancestry (NO `Sort::TIME`): the first commit whose tree
  // contains a path is the one that first added it, so we resolve on that hit.
  rev_walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::REVERSE)?;

  for oid in rev_walk.by_ref() {
    if unresolved.is_empty() {
      break; // early-exit: every path resolved to its creation commit
    }
    // Propagate revwalk iterator errors instead of silently skipping them.
    let oid = oid?;
    // A real object-read failure is an error, not a "no match" -- propagate it.
    let commit = repo.find_commit(oid)?;
    // Probe EVERY commit (merges included -- no `parent_count` branching, no
    // diffing): the exact path either exists in this tree or it doesn't.
    let tree = commit.tree()?;
    for p in unresolved.clone() {
      // NotFound == path absent from this commit's tree (no match); any other
      // lookup error is real and must propagate.
      match tree.get_path(Path::new(&p)) {
        // Only an exact FILE (blob) is a creation. A directory (tree) or submodule
        // (gitlink/Commit) entry is NOT a file: leave the path unresolved so an
        // older blob could still match (a pure directory never will, so it stays
        // `None`/undefined).
        Ok(entry) if entry.kind() == Some(git2::ObjectType::Blob) => {
          result.insert(p.clone(), Some(build_commit_info(&commit)?));
          unresolved.remove(&p);
        }
        Ok(_) => {}
        Err(e) if e.code() == git2::ErrorCode::NotFound => {}
        Err(e) => return Err(e),
      }
    }
  }
  Ok(result)
}

/// Bulk modification lookup enriched with each file's `created` commit. Runs the
/// newest-first bulk modification walk (`get_files_modification`) and the
/// SEPARATE oldest-first bulk creation walk (`get_files_creation`) over ALL input
/// paths, then reconciles per path: a path WITH a modification record gets its
/// `created` merged in; a path with NO record but a creation hit (present at
/// HEAD, touched only by MERGE commits, which the modification walk skips) falls
/// back to the creating commit as the WHOLE record (so `commit_id ==
/// created.commit_id`); a path that no commit ever added stays `None`. The
/// creation walk runs for ALL input paths (not just the ones with a modification
/// record) precisely so those absent-but-existing merge-only paths can fall back;
/// a never-committed path simply resolves to `None` in the creation walk and
/// stays `null`. Shared by the sync method and the async task so both produce
/// identical results.
pub(crate) fn get_files_modification_with_created(
  repo: &git2::Repository,
  filepaths: &[String],
) -> std::result::Result<HashMap<String, Option<FileModification>>, git2::Error> {
  let mut mods = get_files_modification(repo, filepaths)?;
  // Creation for ALL input paths: a merge-only path has NO modification record
  // yet must still fall back to its creating (merge) commit, so we cannot restrict
  // the creation walk to only the paths that already resolved.
  let creations = get_files_creation(repo, filepaths)?;
  for (path, creation) in creations {
    if let Some(slot) = mods.get_mut(&path) {
      match slot {
        // Enrich an existing record with its creation commit.
        Some(fm) => fm.created = creation,
        // No non-merge commit touched this path; if it exists (merge-only), the
        // creating commit becomes the whole record, else it stays `None`.
        None => {
          if let Some(c) = creation {
            *slot = Some(file_modification_from_commit_info(c));
          }
        }
      }
    }
  }
  Ok(mods)
}

/// Newtype over the bulk file-modification map. Its `ToNapiValue` builds the
/// result object with own-property DEFINE semantics (`[[DefineOwnProperty]]` via
/// `napi_define_properties`), NOT `[[Set]]`.
///
/// napi's default `HashMap` serialization uses `napi_set_named_property`
/// (`[[Set]]`), so a valid path key literally named `__proto__` would fire
/// `Object.prototype`'s `__proto__` setter and mutate the RESULT object's
/// prototype instead of creating an own key (present `__proto__` -> its value
/// becomes the prototype; missing -> prototype set to `null`, losing
/// `hasOwnProperty`). Defining each entry as an own enumerable data property
/// bypasses that setter, so EVERY path (including `__proto__`) becomes an own
/// key mapping to its `FileModification` or `null` while the object keeps the
/// normal `Object.prototype`. Surfaces to TS as
/// `Record<string, FileModification | null>` (via the method `ts_return_type`).
pub struct FileModMap(pub(crate) HashMap<String, Option<FileModification>>);

impl TypeName for FileModMap {
  fn type_name() -> &'static str {
    "Object"
  }

  fn value_type() -> ValueType {
    ValueType::Object
  }
}

impl ToNapiValue for FileModMap {
  unsafe fn to_napi_value(
    raw_env: napi::sys::napi_env,
    val: Self,
  ) -> napi::Result<napi::sys::napi_value> {
    let env = Env::from_raw(raw_env);
    let mut obj = Object::new(&env)?;
    let attributes = PropertyAttributes::Enumerable
      | PropertyAttributes::Writable
      | PropertyAttributes::Configurable;
    let properties = val
      .0
      .into_iter()
      .map(|(key, value)| {
        // `Option<FileModification>` ToNapiValue: `None` -> JS `null`,
        // `Some(fm)` -> the FileModification object. `with_utf8_name` errors
        // only on an interior NUL (git paths never contain one).
        Ok(
          Property::new()
            .with_utf8_name(&key)?
            .with_napi_value(&env, value)?
            .with_property_attributes(attributes),
        )
      })
      .collect::<napi::Result<Vec<Property>>>()?;
    obj.define_properties(&properties)?;
    Ok(obj.raw())
  }
}
