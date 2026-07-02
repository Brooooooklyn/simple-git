use std::path::Path;

use chrono::{DateTime, Utc};
use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::error::IntoNapiError;
use crate::{GitErrorCode, Result};

/// Options controlling how a blame is computed.
///
/// Every field is optional; omitted fields fall back to libgit2's defaults
/// (no copy tracking, the whole file, starting from the current HEAD).
#[napi(object)]
#[derive(Clone)]
pub struct BlameOptions {
  /// Track lines that have moved within a file. Defaults to `false`.
  ///
  /// Note: libgit2 1.9.4 does not implement blame copy/move tracking, so
  /// setting this flag has no effect today (accepted for forward-compat;
  /// effectively a no-op).
  pub track_copies_same_file: Option<bool>,
  /// Track lines that have moved across files in the same commit. Defaults to `false`.
  ///
  /// Note: libgit2 1.9.4 does not implement blame copy/move tracking, so
  /// setting this flag has no effect today (accepted for forward-compat;
  /// effectively a no-op).
  pub track_copies_same_commit_moves: Option<bool>,
  /// 40-char hex OID of the newest commit to consider (the blame starts here).
  pub newest_commit: Option<String>,
  /// 40-char hex OID of the oldest commit to consider (a boundary).
  pub oldest_commit: Option<String>,
  /// Restrict the search to first-parent history only. Defaults to `false`.
  pub first_parent: Option<bool>,
  /// Map names/emails through the repository's mailmap. Defaults to `false`.
  pub use_mailmap: Option<bool>,
  /// Ignore whitespace differences. Defaults to `false`.
  pub ignore_whitespace: Option<bool>,
  /// The first line in the file to blame (1-based).
  pub min_line: Option<u32>,
  /// The last line in the file to blame (1-based).
  pub max_line: Option<u32>,
}

/// A single blame hunk: a contiguous run of lines attributed to one commit.
///
/// All identity fields are copied out of the borrowed `git2::BlameHunk` so the
/// value can safely outlive the underlying `git2::Blame`.
#[napi(object)]
pub struct BlameHunk {
  /// Number of lines covered by this hunk.
  pub lines_in_hunk: u32,
  /// 40-char lowercase hex OID of the commit where these lines were last changed.
  pub final_commit_id: String,
  /// Line number where this hunk begins in the final file (1-based).
  pub final_start_line: u32,
  /// Author name of the final commit. Undefined if absent or not valid UTF-8.
  pub final_author_name: Option<String>,
  /// Author email of the final commit. Undefined if absent or not valid UTF-8.
  pub final_author_email: Option<String>,
  /// Author time of the final commit, as a `Date`. The Unix epoch if no signature.
  pub final_time: DateTime<Utc>,
  /// 40-char lowercase hex OID of the commit where this hunk was found.
  pub orig_commit_id: String,
  /// Line number where this hunk begins in the original file (1-based).
  pub orig_start_line: u32,
  /// Path to the file where this hunk originated. Undefined if not valid UTF-8.
  pub orig_path: Option<String>,
  /// Whether this hunk was tracked to a boundary commit (root or `oldest_commit`).
  pub is_boundary: bool,
}

/// Translate the optional `BlameOptions` into a `git2::BlameOptions` builder.
///
/// Returns a napi error when `newest_commit`/`oldest_commit` is not a valid OID.
pub(crate) fn build_blame_opts(opts: Option<BlameOptions>) -> Result<git2::BlameOptions> {
  let mut builder = git2::BlameOptions::new();
  let Some(opts) = opts else {
    return Ok(builder);
  };
  if let Some(v) = opts.track_copies_same_file {
    builder.track_copies_same_file(v);
  }
  if let Some(v) = opts.track_copies_same_commit_moves {
    builder.track_copies_same_commit_moves(v);
  }
  if let Some(v) = opts.first_parent {
    builder.first_parent(v);
  }
  if let Some(v) = opts.use_mailmap {
    builder.use_mailmap(v);
  }
  if let Some(v) = opts.ignore_whitespace {
    builder.ignore_whitespace(v);
  }
  if let Some(oid) = opts.newest_commit {
    builder.newest_commit(
      git2::Oid::from_str(&oid).convert(format!("Invalid newest_commit OID [{oid}]"))?,
    );
  }
  if let Some(oid) = opts.oldest_commit {
    builder.oldest_commit(
      git2::Oid::from_str(&oid).convert(format!("Invalid oldest_commit OID [{oid}]"))?,
    );
  }
  if let Some(v) = opts.min_line {
    builder.min_line(v as usize);
  }
  if let Some(v) = opts.max_line {
    builder.max_line(v as usize);
  }
  Ok(builder)
}

/// Eagerly copy a borrowed `git2::BlameHunk` into an owned `BlameHunk`.
///
/// The author identity is read from `final_signature()` (which may be absent,
/// e.g. for in-memory buffer blames); `final_time` is a `Date`, falling back to
/// the Unix epoch when no signature is present.
pub(crate) fn hunk_to_struct(hunk: &git2::BlameHunk) -> Result<BlameHunk> {
  let (final_author_name, final_author_email, final_seconds) = match hunk.final_signature() {
    Some(sig) => (
      sig.name().ok().map(|s| s.to_owned()),
      sig.email().ok().map(|s| s.to_owned()),
      sig.when().seconds(),
    ),
    None => (None, None, 0),
  };
  let final_time = DateTime::from_timestamp(final_seconds, 0)
    .ok_or_else(|| Error::new(GitErrorCode::GenericError, "Invalid blame final time"))?;
  Ok(BlameHunk {
    lines_in_hunk: hunk.lines_in_hunk() as u32,
    final_commit_id: hunk.final_commit_id().to_string(),
    final_start_line: hunk.final_start_line() as u32,
    final_author_name,
    final_author_email,
    final_time,
    orig_commit_id: hunk.orig_commit_id().to_string(),
    orig_start_line: hunk.orig_start_line() as u32,
    // git2 0.21 exposes the originating path as `BlameHunk::path()`; non-UTF-8
    // paths collapse to `None`.
    orig_path: hunk.path().and_then(|p| p.to_str()).map(|s| s.to_owned()),
    is_boundary: hunk.is_boundary(),
  })
}

/// Compute the blame for `path` and eagerly materialize every hunk so nothing
/// borrowing the repository (the `Blame<'repo>`/`BlameHunk<'blame>`) escapes.
pub(crate) fn collect_blame(
  repo: &git2::Repository,
  path: &str,
  options: Option<BlameOptions>,
) -> Result<Vec<BlameHunk>> {
  let mut opts = build_blame_opts(options)?;
  let blame = repo
    .blame_file(Path::new(path), Some(&mut opts))
    .convert_without_message()?;
  blame.iter().map(|hunk| hunk_to_struct(&hunk)).collect()
}

/// Compute the blame for `path` and return the single hunk covering `line_no`
/// (1-based), eagerly copied out before the borrowed blame drops.
pub(crate) fn blame_single_line(
  repo: &git2::Repository,
  path: &str,
  line_no: u32,
  options: Option<BlameOptions>,
) -> Result<Option<BlameHunk>> {
  let mut opts = build_blame_opts(options)?;
  let blame = repo
    .blame_file(Path::new(path), Some(&mut opts))
    .convert_without_message()?;
  blame
    .get_line(line_no as usize)
    .map(|hunk| hunk_to_struct(&hunk))
    .transpose()
}
