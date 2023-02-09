use std::borrow::Borrow;
use std::path::{Path, PathBuf};

use napi::{
  bindgen_prelude::{
    AbortSignal, AsyncTask, Env, Error, Reference, Result, Status, Task, ToNapiValue,
  },
  JsString,
};
use napi_derive::napi;
use once_cell::sync::Lazy;

use crate::diff::Diff;
use crate::error::{IntoNapiError, NotNullError};
use crate::reference;
use crate::remote::Remote;
use crate::signature::Signature;
use crate::tree::{Tree, TreeParent};

static INIT_GIT_CONFIG: Lazy<Result<()>> = Lazy::new(|| {
  // Handle the `failed to stat '/root/.gitconfig'; class=Config (7)` Error
  #[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
  ))]
  {
    if git2::Config::find_global().is_err() {
      if let Some(mut git_config_dir) = dirs::home_dir() {
        git_config_dir.push(".gitconfig");
        std::fs::write(&git_config_dir, "").map_err(|err| {
          Error::new(
            Status::GenericFailure,
            format!("Initialize {:?} failed {}", git_config_dir, err),
          )
        })?;
      }
    }
  }
  Ok(())
});

#[napi]
pub enum RepositoryState {
  Clean,
  Merge,
  Revert,
  RevertSequence,
  CherryPick,
  CherryPickSequence,
  Bisect,
  Rebase,
  RebaseInteractive,
  RebaseMerge,
  ApplyMailbox,
  ApplyMailboxOrRebase,
}

impl From<git2::RepositoryState> for RepositoryState {
  fn from(value: git2::RepositoryState) -> Self {
    match value {
      git2::RepositoryState::ApplyMailbox => Self::ApplyMailbox,
      git2::RepositoryState::ApplyMailboxOrRebase => Self::ApplyMailboxOrRebase,
      git2::RepositoryState::Bisect => Self::Bisect,
      git2::RepositoryState::Rebase => Self::Rebase,
      git2::RepositoryState::RebaseInteractive => Self::RebaseInteractive,
      git2::RepositoryState::RebaseMerge => Self::RebaseMerge,
      git2::RepositoryState::CherryPick => Self::CherryPick,
      git2::RepositoryState::CherryPickSequence => Self::CherryPickSequence,
      git2::RepositoryState::Merge => Self::Merge,
      git2::RepositoryState::Revert => Self::Revert,
      git2::RepositoryState::RevertSequence => Self::RevertSequence,
      git2::RepositoryState::Clean => Self::Clean,
    }
  }
}

#[napi]
pub enum RepositoryOpenFlags {
  /// Only open the specified path; don't walk upward searching.
  NoSearch,
  /// Search across filesystem boundaries.
  CrossFS,
  /// Force opening as bare repository, and defer loading its config.
  Bare,
  /// Don't try appending `/.git` to the specified repository path.
  NoDotGit,
  /// Respect environment variables like `$GIT_DIR`.
  FromEnv,
}

impl From<RepositoryOpenFlags> for git2::RepositoryOpenFlags {
  fn from(val: RepositoryOpenFlags) -> Self {
    match val {
      RepositoryOpenFlags::NoSearch => git2::RepositoryOpenFlags::NO_SEARCH,
      RepositoryOpenFlags::CrossFS => git2::RepositoryOpenFlags::CROSS_FS,
      RepositoryOpenFlags::Bare => git2::RepositoryOpenFlags::BARE,
      RepositoryOpenFlags::NoDotGit => git2::RepositoryOpenFlags::NO_DOTGIT,
      RepositoryOpenFlags::FromEnv => git2::RepositoryOpenFlags::FROM_ENV,
    }
  }
}

pub struct GitDateTask {
  repo: napi::bindgen_prelude::Reference<Repository>,
  filepath: String,
}

#[napi]
impl Task for GitDateTask {
  type Output = i64;
  type JsValue = i64;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    get_file_modified_date(&self.repo.inner, &self.filepath)
      .convert_without_message()
      .and_then(|value| {
        value.expect_not_null(format!("Failed to get commit for [{}]", &self.filepath))
      })
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
pub struct Repository {
  pub(crate) inner: git2::Repository,
}

#[napi]
impl Repository {
  #[napi]
  pub fn init(p: String) -> Result<Repository> {
    INIT_GIT_CONFIG.as_ref().map_err(|err| err.clone())?;
    Ok(Self {
      inner: git2::Repository::init(&p).map_err(|err| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to open git repo: [{p}], reason: {err}",),
        )
      })?,
    })
  }

  #[napi]
  /// Find and open an existing repository, with additional options.
  ///
  /// If flags contains REPOSITORY_OPEN_NO_SEARCH, the path must point
  /// directly to a repository; otherwise, this may point to a subdirectory
  /// of a repository, and `open_ext` will search up through parent
  /// directories.
  ///
  /// If flags contains REPOSITORY_OPEN_CROSS_FS, the search through parent
  /// directories will not cross a filesystem boundary (detected when the
  /// stat st_dev field changes).
  ///
  /// If flags contains REPOSITORY_OPEN_BARE, force opening the repository as
  /// bare even if it isn't, ignoring any working directory, and defer
  /// loading the repository configuration for performance.
  ///
  /// If flags contains REPOSITORY_OPEN_NO_DOTGIT, don't try appending
  /// `/.git` to `path`.
  ///
  /// If flags contains REPOSITORY_OPEN_FROM_ENV, `open_ext` will ignore
  /// other flags and `ceiling_dirs`, and respect the same environment
  /// variables git does. Note, however, that `path` overrides `$GIT_DIR`; to
  /// respect `$GIT_DIR` as well, use `open_from_env`.
  ///
  /// ceiling_dirs specifies a list of paths that the search through parent
  /// directories will stop before entering.  Use the functions in std::env
  /// to construct or manipulate such a path list.
  pub fn open_ext(
    path: String,
    flags: RepositoryOpenFlags,
    ceiling_dirs: Vec<String>,
  ) -> Result<Repository> {
    INIT_GIT_CONFIG.as_ref().map_err(|err| err.clone())?;
    Ok(Self {
      inner: git2::Repository::open_ext(path, flags.into(), ceiling_dirs)
        .convert("Failed to open git repo")?,
    })
  }

  #[napi]
  /// Attempt to open an already-existing repository at or above `path`
  ///
  /// This starts at `path` and looks up the filesystem hierarchy
  /// until it finds a repository.
  pub fn discover(path: String) -> Result<Repository> {
    INIT_GIT_CONFIG.as_ref().map_err(|err| err.clone())?;
    Ok(Self {
      inner: git2::Repository::discover(&path)
        .convert(format!("Discover git repo from [{path}] failed"))?,
    })
  }

  #[napi(constructor)]
  pub fn new(git_dir: String) -> Result<Self> {
    INIT_GIT_CONFIG.as_ref().map_err(|err| err.clone())?;
    Ok(Self {
      inner: git2::Repository::open(&git_dir).map_err(|err| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to open git repo: [{git_dir}], reason: {err}",),
        )
      })?,
    })
  }

  #[napi]
  /// Retrieve and resolve the reference pointed at by HEAD.
  pub fn head(&self, self_ref: Reference<Repository>, env: Env) -> Result<reference::Reference> {
    Ok(reference::Reference {
      inner: self_ref.share_with(env, |repo| {
        repo
          .inner
          .head()
          .convert("Get the HEAD of Repository failed")
      })?,
    })
  }

  #[napi]
  pub fn is_shallow(&self) -> Result<bool> {
    Ok(self.inner.is_shallow())
  }

  #[napi]
  pub fn is_empty(&self) -> Result<bool> {
    self.inner.is_empty().convert_without_message()
  }

  #[napi]
  pub fn is_worktree(&self) -> Result<bool> {
    Ok(self.inner.is_worktree())
  }

  #[napi]
  /// Returns the path to the `.git` folder for normal repositories or the
  /// repository itself for bare repositories.
  pub fn path(&self, env: Env) -> Result<JsString> {
    path_to_javascript_string(&env, self.inner.path())
  }

  #[napi]
  /// Returns the current state of this repository
  pub fn state(&self) -> Result<RepositoryState> {
    Ok(self.inner.state().into())
  }

  #[napi]
  /// Get the path of the working directory for this repository.
  ///
  /// If this repository is bare, then `None` is returned.
  pub fn workdir(&self, env: Env) -> Option<JsString> {
    self
      .inner
      .workdir()
      .and_then(|path| path_to_javascript_string(&env, path).ok())
  }

  #[napi]
  /// Set the path to the working directory for this repository.
  ///
  /// If `update_link` is true, create/update the gitlink file in the workdir
  /// and set config "core.worktree" (if workdir is not the parent of the .git
  /// directory).
  pub fn set_workdir(&self, path: String, update_gitlink: bool) -> Result<()> {
    self
      .inner
      .set_workdir(PathBuf::from(path).as_path(), update_gitlink)
      .convert_without_message()?;
    Ok(())
  }

  #[napi]
  /// Get the currently active namespace for this repository.
  ///
  /// If there is no namespace, or the namespace is not a valid utf8 string,
  /// `None` is returned.
  pub fn namespace(&self) -> Option<String> {
    self.inner.namespace().map(|n| n.to_owned())
  }

  #[napi]
  /// Set the active namespace for this repository.
  pub fn set_namespace(&self, namespace: String) -> Result<()> {
    self
      .inner
      .set_namespace(&namespace)
      .convert_without_message()?;
    Ok(())
  }

  #[napi]
  /// Remove the active namespace for this repository.
  pub fn remove_namespace(&self) -> Result<()> {
    self.inner.remove_namespace().convert_without_message()?;
    Ok(())
  }

  #[napi]
  /// Retrieves the Git merge message.
  /// Remember to remove the message when finished.
  pub fn message(&self) -> Result<String> {
    self
      .inner
      .message()
      .convert("Failed to get Git merge message")
  }

  #[napi]
  /// Remove the Git merge message.
  pub fn remove_message(&self) -> Result<()> {
    self
      .inner
      .remove_message()
      .convert("Remove the Git merge message failed")
  }

  #[napi]
  /// List all remotes for a given repository
  pub fn remotes(&self) -> Result<Vec<String>> {
    self
      .inner
      .remotes()
      .map(|remotes| {
        remotes
          .into_iter()
          .flatten()
          .map(|name| name.to_owned())
          .collect()
      })
      .convert("Fetch remotes failed")
  }

  #[napi]
  /// Get the information for a particular remote
  pub fn remote(&self, self_ref: Reference<Repository>, env: Env, name: String) -> Result<Remote> {
    Ok(Remote {
      inner: self_ref.share_with(env, move |repo| {
        repo
          .inner
          .find_remote(&name)
          .convert(format!("Failed to get remote [{}]", &name))
      })?,
    })
  }

  #[napi]
  /// Lookup a reference to one of the objects in a repository.
  pub fn find_tree(&self, oid: String, self_ref: Reference<Repository>, env: Env) -> Result<Tree> {
    Ok(Tree {
      inner: TreeParent::Repository(self_ref.share_with(env, |repo| {
        repo
          .inner
          .find_tree(git2::Oid::from_str(oid.as_str()).convert(format!("Invalid OID [{oid}]"))?)
          .convert(format!("Find tree from OID [{oid}] failed"))
      })?),
    })
  }

  #[napi]
  /// Create a diff between a tree and the working directory.
  ///
  /// The tree you provide will be used for the "old_file" side of the delta,
  /// and the working directory will be used for the "new_file" side.
  ///
  /// This is not the same as `git diff <treeish>` or `git diff-index
  /// <treeish>`.  Those commands use information from the index, whereas this
  /// function strictly returns the differences between the tree and the files
  /// in the working directory, regardless of the state of the index.  Use
  /// `tree_to_workdir_with_index` to emulate those commands.
  ///
  /// To see difference between this and `tree_to_workdir_with_index`,
  /// consider the example of a staged file deletion where the file has then
  /// been put back into the working dir and further modified.  The
  /// tree-to-workdir diff for that file is 'modified', but `git diff` would
  /// show status 'deleted' since there is a staged delete.
  ///
  /// If `None` is passed for `tree`, then an empty tree is used.
  pub fn diff_tree_to_workdir(
    &self,
    env: Env,
    self_reference: Reference<Repository>,
    old_tree: Option<&Tree>,
  ) -> Result<Diff> {
    let mut diff_options = git2::DiffOptions::default();
    Ok(Diff {
      inner: self_reference.share_with(env, |repo| {
        repo
          .inner
          .diff_tree_to_workdir(old_tree.map(|t| t.inner()), Some(&mut diff_options))
          .convert_without_message()
      })?,
    })
  }

  #[napi]
  /// Create a diff between a tree and the working directory using index data
  /// to account for staged deletes, tracked files, etc.
  ///
  /// This emulates `git diff <tree>` by diffing the tree to the index and
  /// the index to the working directory and blending the results into a
  /// single diff that includes staged deleted, etc.
  pub fn diff_tree_to_workdir_with_index(
    &self,
    env: Env,
    self_reference: Reference<Repository>,
    old_tree: Option<&Tree>,
  ) -> Result<Diff> {
    let mut diff_options = git2::DiffOptions::default();
    Ok(Diff {
      inner: self_reference.share_with(env, |repo| {
        repo
          .inner
          .diff_tree_to_workdir_with_index(old_tree.map(|t| t.inner()), Some(&mut diff_options))
          .convert_without_message()
      })?,
    })
  }

  #[napi]
  /// Create new commit in the repository
  ///
  /// If the `update_ref` is not `None`, name of the reference that will be
  /// updated to point to this commit. If the reference is not direct, it will
  /// be resolved to a direct reference. Use "HEAD" to update the HEAD of the
  /// current branch and make it point to this commit. If the reference
  /// doesn't exist yet, it will be created. If it does exist, the first
  /// parent must be the tip of this branch.
  pub fn commit(
    &self,
    update_ref: Option<String>,
    author: &Signature,
    committer: &Signature,
    message: String,
    tree: &Tree,
  ) -> Result<String> {
    self
      .inner
      .commit(
        update_ref.as_deref(),
        author.as_ref(),
        committer.as_ref(),
        message.as_str(),
        tree.as_ref(),
        &[],
      )
      .convert_without_message()
      .map(|oid| oid.to_string())
  }

  #[napi]
  pub fn get_file_latest_modified_date(&self, filepath: String) -> Result<i64> {
    get_file_modified_date(&self.inner, &filepath)
      .convert_without_message()
      .and_then(|value| value.expect_not_null(format!("Failed to get commit for [{filepath}]")))
  }

  #[napi]
  pub fn get_file_latest_modified_date_async(
    &self,
    self_ref: Reference<Repository>,
    filepath: String,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<GitDateTask>> {
    Ok(AsyncTask::with_optional_signal(
      GitDateTask {
        repo: self_ref,
        filepath,
      },
      signal,
    ))
  }
}

fn get_file_modified_date(
  repo: &git2::Repository,
  filepath: &str,
) -> std::result::Result<Option<i64>, git2::Error> {
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
              {
                if diff.deltas().len() > 0 {
                  return Some(commit.time().seconds() * 1000);
                }
              }
            }
          }
          // root commit
          0 => {
            let tree = commit.tree().ok()?;
            if tree.get_path(&path).is_ok() {
              return Some(commit.time().seconds() * 1000);
            }
          }
          // ignore merge commits
          _ => {}
        };
        None
      }),
  )
}

fn path_to_javascript_string(env: &Env, p: &Path) -> Result<JsString> {
  #[cfg(unix)]
  {
    let path = p.to_string_lossy();
    env.create_string(path.borrow())
  }
  #[cfg(windows)]
  {
    use std::os::windows::ffi::OsStrExt;
    let path_buf = p.as_os_str().encode_wide().collect::<Vec<u16>>();
    env.create_string_utf16(path_buf.as_slice())
  }
}
