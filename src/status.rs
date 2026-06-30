use napi_derive::napi;

/// Options controlling how a working-tree status scan is performed.
///
/// Every field is optional; omitted fields fall back to the git CLI defaults
/// (`include_untracked` is `true`, everything else `false`).
#[napi(object)]
#[derive(Clone)]
pub struct StatusOptions {
  /// Include untracked files in the status. Defaults to `true`.
  pub include_untracked: Option<bool>,
  /// Include ignored files in the status. Defaults to `false`.
  pub include_ignored: Option<bool>,
  /// Include unmodified files in the status. Defaults to `false`.
  pub include_unmodified: Option<bool>,
  /// Skip submodules. Defaults to `false`.
  pub exclude_submodules: Option<bool>,
  /// Recurse into untracked directories instead of reporting the directory
  /// itself. Defaults to `false`.
  pub recurse_untracked_dirs: Option<bool>,
  /// Detect renames between the HEAD tree and the index. Defaults to `false`.
  pub renames_head_to_index: Option<bool>,
  /// Detect renames between the index and the working directory. Defaults to `false`.
  pub renames_index_to_workdir: Option<bool>,
  /// Restrict the scan to the given pathspecs.
  pub pathspec: Option<Vec<String>>,
}

/// Status of a single file in the working tree and/or index.
///
/// The boolean flags mirror the `git2::Status` bits; `bits` carries the raw
/// value as a forward-compatible escape hatch for flags not surfaced here.
#[napi(object)]
pub struct FileStatus {
  /// Workdir-relative path. `null` if the path is not valid UTF-8.
  pub path: Option<String>,
  /// Raw `git2::Status` bits — forward-compat escape hatch.
  pub bits: u32,
  /// Staged: a new file was added to the index.
  pub is_index_new: bool,
  /// Staged: a tracked file was modified in the index.
  pub is_index_modified: bool,
  /// Staged: a tracked file was deleted from the index.
  pub is_index_deleted: bool,
  /// Staged: a tracked file was renamed in the index.
  pub is_index_renamed: bool,
  /// Staged: a tracked file changed type in the index.
  pub is_index_typechange: bool,
  /// Unstaged: an untracked file (new in the working directory).
  pub is_wt_new: bool,
  /// Unstaged: a tracked file was modified in the working directory.
  pub is_wt_modified: bool,
  /// Unstaged: a tracked file was deleted from the working directory.
  pub is_wt_deleted: bool,
  /// Unstaged: a tracked file changed type in the working directory.
  pub is_wt_typechange: bool,
  /// Unstaged: a tracked file was renamed in the working directory.
  pub is_wt_renamed: bool,
  /// The file is ignored.
  pub is_ignored: bool,
  /// The file has merge conflicts.
  pub is_conflicted: bool,
}

/// Translate the optional `StatusOptions` into a `git2::StatusOptions` builder.
///
/// Defaults match the git CLI: `include_untracked` is enabled unless explicitly
/// disabled; every other flag is opt-in.
pub(crate) fn build_status_opts(opts: Option<StatusOptions>) -> git2::StatusOptions {
  let mut builder = git2::StatusOptions::new();
  let opts = opts.unwrap_or(StatusOptions {
    include_untracked: None,
    include_ignored: None,
    include_unmodified: None,
    exclude_submodules: None,
    recurse_untracked_dirs: None,
    renames_head_to_index: None,
    renames_index_to_workdir: None,
    pathspec: None,
  });
  builder.include_untracked(opts.include_untracked.unwrap_or(true));
  builder.include_ignored(opts.include_ignored.unwrap_or(false));
  builder.include_unmodified(opts.include_unmodified.unwrap_or(false));
  builder.exclude_submodules(opts.exclude_submodules.unwrap_or(false));
  builder.recurse_untracked_dirs(opts.recurse_untracked_dirs.unwrap_or(false));
  builder.renames_head_to_index(opts.renames_head_to_index.unwrap_or(false));
  builder.renames_index_to_workdir(opts.renames_index_to_workdir.unwrap_or(false));
  if let Some(pathspec) = opts.pathspec {
    for p in pathspec {
      builder.pathspec(p);
    }
  }
  builder
}

/// Decode a `git2::Status` into a `FileStatus`, preserving the raw bits.
pub(crate) fn status_from_bits(status: git2::Status, path: Option<String>) -> FileStatus {
  FileStatus {
    path,
    bits: status.bits(),
    is_index_new: status.is_index_new(),
    is_index_modified: status.is_index_modified(),
    is_index_deleted: status.is_index_deleted(),
    is_index_renamed: status.is_index_renamed(),
    is_index_typechange: status.is_index_typechange(),
    is_wt_new: status.is_wt_new(),
    is_wt_modified: status.is_wt_modified(),
    is_wt_deleted: status.is_wt_deleted(),
    is_wt_typechange: status.is_wt_typechange(),
    is_wt_renamed: status.is_wt_renamed(),
    is_ignored: status.is_ignored(),
    is_conflicted: status.is_conflicted(),
  }
}
