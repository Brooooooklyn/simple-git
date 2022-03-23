use std::borrow::Borrow;
use std::path::{Path, PathBuf};

use napi::{
  bindgen_prelude::{AbortSignal, AsyncTask, Env, Error, Result, Status, Task, ToNapiValue},
  JsString,
};
use napi_derive::napi;
use once_cell::sync::Lazy;

use crate::error::{IntoNapiError, NotNullError};
use crate::reference::Reference;
use crate::remote::Remote;

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
    Ok(Self {
      inner: git2::Repository::init(&p).map_err(|err| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to open git repo: [{}], reason: {}", p, err,),
        )
      })?,
    })
  }

  #[napi(constructor)]
  pub fn new(git_dir: String) -> Result<Self> {
    INIT_GIT_CONFIG.as_ref().map_err(|err| err.clone())?;
    Ok(Self {
      inner: git2::Repository::open(&git_dir).map_err(|err| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to open git repo: [{}], reason: {}", git_dir, err,),
        )
      })?,
    })
  }

  #[napi]
  /// Retrieve and resolve the reference pointed at by HEAD.
  pub fn head(&self) -> Result<Reference> {
    Ok(crate::reference::Reference {
      inner: self.create_reference()?.share_with(|repo| {
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
          .filter_map(|name| name)
          .map(|name| name.to_owned())
          .collect()
      })
      .convert("Fetch remotes failed")
  }

  #[napi]
  /// Get the information for a particular remote
  pub fn remote(&self, name: String) -> Result<Remote> {
    Ok(Remote {
      inner: self.create_reference()?.share_with(move |repo| {
        repo
          .inner
          .find_remote(&name)
          .convert(format!("Failed to get remote [{}]", &name))
      })?,
    })
  }

  #[napi]
  pub fn get_file_latest_modified_date(&self, filepath: String) -> Result<i64> {
    get_file_modified_date(&self.inner, &filepath)
      .convert_without_message()
      .and_then(|value| value.expect_not_null(format!("Failed to get commit for [{}]", filepath)))
  }

  #[napi]
  pub fn get_file_latest_modified_date_async(
    &self,
    filepath: String,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<GitDateTask>> {
    Ok(AsyncTask::with_optional_signal(
      GitDateTask {
        repo: self.create_reference()?,
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
