use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use napi::{JsString, bindgen_prelude::*};
use napi_derive::napi;
use once_cell::sync::Lazy;

use crate::blame::{BlameHunk, BlameOptions, blame_single_line, collect_blame};
use crate::branch::{Branch, BranchType};
use crate::checkout::{CheckoutOptions, build_checkout_builder};
use crate::commit::{Commit, CommitInner};
use crate::config::Config;
use crate::diff::Diff;
use crate::error::{IntoNapiError, NotNullError};
use crate::file_modification::{FileModification, get_file_modification, get_files_modification};
use crate::index::Index;
use crate::object::{GitObject, ObjectParent};
use crate::reference;
use crate::remote::Remote;
use crate::rev_walk::RevWalk;
use crate::signature::Signature;
use crate::status::{FileStatus, StatusOptions, build_status_opts, status_from_bits};
use crate::tag::Tag;
use crate::tree::{Tree, TreeEntry, TreeParent};
use crate::util::path_to_javascript_string;

static INIT_GIT_CONFIG: Lazy<Result<()>> = Lazy::new(|| {
  // Handle the `failed to stat '/root/.gitconfig'; class=Config (7)` Error
  #[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
  ))]
  {
    if git2::Config::find_global().is_err()
      && let Some(mut git_config_dir) = dirs::home_dir()
    {
      git_config_dir.push(".gitconfig");
      std::fs::write(&git_config_dir, "").map_err(|err| {
        Error::new(
          Status::GenericFailure,
          format!("Initialize {git_config_dir:?} failed {err}"),
        )
      })?;
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
  repo: RwLock<napi::bindgen_prelude::Reference<Repository>>,
  filepath: String,
}

unsafe impl Send for GitDateTask {}

pub struct GitCreatedDateTask {
  repo: RwLock<napi::bindgen_prelude::Reference<Repository>>,
  filepath: String,
}

unsafe impl Send for GitCreatedDateTask {}

pub struct GitModificationTask {
  repo: RwLock<napi::bindgen_prelude::Reference<Repository>>,
  filepath: String,
}

unsafe impl Send for GitModificationTask {}

pub struct GitBulkModificationTask {
  repo: RwLock<napi::bindgen_prelude::Reference<Repository>>,
  filepaths: Vec<String>,
}

unsafe impl Send for GitBulkModificationTask {}

pub struct GitStatusTask {
  repo: RwLock<napi::bindgen_prelude::Reference<Repository>>,
  options: Option<StatusOptions>,
}

unsafe impl Send for GitStatusTask {}

pub struct GitBlameTask {
  repo: RwLock<napi::bindgen_prelude::Reference<Repository>>,
  filepath: String,
  options: Option<BlameOptions>,
}

unsafe impl Send for GitBlameTask {}

#[napi]
impl Task for GitDateTask {
  type Output = i64;
  type JsValue = i64;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    get_file_modification(
      &self
        .repo
        .read()
        .map_err(|err| napi::Error::new(Status::GenericFailure, format!("{err}")))?
        .inner,
      &self.filepath,
    )
    .convert_without_message()
    .and_then(|value| {
      value
        .map(|m| m.timestamp)
        .expect_not_null(format!("Failed to get commit for [{}]", &self.filepath))
    })
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
impl Task for GitCreatedDateTask {
  type Output = i64;
  type JsValue = i64;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    get_file_created_date(
      &self
        .repo
        .read()
        .map_err(|err| napi::Error::new(Status::GenericFailure, format!("{err}")))?
        .inner,
      &self.filepath,
    )
    .convert_without_message()
    .and_then(|value| {
      value.expect_not_null(format!(
        "Failed to get created date for [{}]",
        &self.filepath
      ))
    })
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
impl Task for GitModificationTask {
  type Output = Option<FileModification>;
  type JsValue = Option<FileModification>;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    get_file_modification(
      &self
        .repo
        .read()
        .map_err(|err| napi::Error::new(Status::GenericFailure, format!("{err}")))?
        .inner,
      &self.filepath,
    )
    .convert_without_message()
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
impl Task for GitBulkModificationTask {
  type Output = HashMap<String, Option<FileModification>>;
  type JsValue = HashMap<String, Option<FileModification>>;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    get_files_modification(
      &self
        .repo
        .read()
        .map_err(|err| napi::Error::new(Status::GenericFailure, format!("{err}")))?
        .inner,
      &self.filepaths,
    )
    .convert_without_message()
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
impl Task for GitStatusTask {
  type Output = Vec<FileStatus>;
  type JsValue = Vec<FileStatus>;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    collect_statuses(
      &self
        .repo
        .read()
        .map_err(|err| napi::Error::new(Status::GenericFailure, format!("{err}")))?
        .inner,
      self.options.clone(),
    )
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
impl Task for GitBlameTask {
  type Output = Vec<BlameHunk>;
  type JsValue = Vec<BlameHunk>;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    collect_blame(
      &self
        .repo
        .read()
        .map_err(|err| napi::Error::new(Status::GenericFailure, format!("{err}")))?
        .inner,
      &self.filepath,
      self.options.clone(),
    )
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
  #[napi(factory)]
  pub fn init(p: String) -> Result<Repository> {
    INIT_GIT_CONFIG
      .as_ref()
      .map_err(|err| Error::new(err.status, err.reason.clone()))?;
    Ok(Self {
      inner: git2::Repository::init(&p).map_err(|err| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to open git repo: [{p}], reason: {err}",),
        )
      })?,
    })
  }

  #[napi(factory)]
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
    INIT_GIT_CONFIG
      .as_ref()
      .map_err(|err| Error::new(err.status, err.reason.clone()))?;
    Ok(Self {
      inner: git2::Repository::open_ext(path, flags.into(), ceiling_dirs)
        .convert("Failed to open git repo")?,
    })
  }

  #[napi(factory)]
  /// Attempt to open an already-existing repository at or above `path`
  ///
  /// This starts at `path` and looks up the filesystem hierarchy
  /// until it finds a repository.
  pub fn discover(path: String) -> Result<Repository> {
    INIT_GIT_CONFIG
      .as_ref()
      .map_err(|err| Error::new(err.status, err.reason.clone()))?;
    Ok(Self {
      inner: git2::Repository::discover(&path)
        .convert(format!("Discover git repo from [{path}] failed"))?,
    })
  }

  #[napi(factory)]
  /// Creates a new `--bare` repository in the specified folder.
  ///
  /// The folder must exist prior to invoking this function.
  pub fn init_bare(path: String) -> Result<Self> {
    Ok(Self {
      inner: git2::Repository::init_bare(path).convert("Failed to init bare repo")?,
    })
  }

  #[napi(factory)]
  /// Clone a remote repository.
  ///
  /// See the `RepoBuilder` struct for more information. This function will
  /// delegate to a fresh `RepoBuilder`
  pub fn clone(url: String, path: String) -> Result<Self> {
    Ok(Self {
      inner: git2::Repository::clone(&url, path).convert("Failed to clone repo")?,
    })
  }

  #[napi(factory)]
  /// Clone a remote repository, initialize and update its submodules
  /// recursively.
  ///
  /// This is similar to `git clone --recursive`.
  pub fn clone_recurse(url: String, path: String) -> Result<Self> {
    Ok(Self {
      inner: git2::Repository::clone_recurse(&url, path)
        .convert("Failed to clone repo recursively")?,
    })
  }

  #[napi(constructor)]
  /// Attempt to open an already-existing repository at `path`.
  ///
  /// The path can point to either a normal or bare repository.
  pub fn new(git_dir: String) -> Result<Self> {
    INIT_GIT_CONFIG
      .as_ref()
      .map_err(|err| Error::new(err.status, err.reason.clone()))?;
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
  /// Get the configuration file for this repository.
  ///
  /// If a configuration file has not been set, the default config set for the
  /// repository will be returned, including global and system configurations.
  pub fn config(&self) -> Result<Config> {
    Ok(Config {
      inner: self.inner.config().convert_without_message()?,
    })
  }

  #[napi]
  /// Create a new action signature with default user and now timestamp.
  ///
  /// This looks up the `user.name` and `user.email` from the configuration and
  /// uses the current time as the timestamp. It returns an error if either the
  /// `user.name` or `user.email` are not set.
  pub fn signature(&self) -> Result<Signature> {
    Ok(Signature::from_git2(
      self.inner.signature().convert_without_message()?,
    ))
  }

  #[napi]
  /// Tests whether this repository is a shallow clone.
  pub fn is_shallow(&self) -> Result<bool> {
    Ok(self.inner.is_shallow())
  }

  #[napi]
  /// Tests whether this repository is empty.
  pub fn is_empty(&self) -> Result<bool> {
    self.inner.is_empty().convert_without_message()
  }

  #[napi]
  /// Tests whether this repository is a worktree.
  pub fn is_worktree(&self) -> Result<bool> {
    Ok(self.inner.is_worktree())
  }

  #[napi]
  /// Returns the path to the `.git` folder for normal repositories or the
  /// repository itself for bare repositories.
  pub fn path<'env>(&'env self, env: &'env Env) -> Result<JsString<'env>> {
    path_to_javascript_string(env, self.inner.path())
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
  pub fn workdir<'env>(&'env self, env: &'env Env) -> Option<JsString<'env>> {
    self
      .inner
      .workdir()
      .and_then(move |path| path_to_javascript_string(env, path).ok())
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
    self.inner.namespace().ok().flatten().map(|n| n.to_owned())
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
          .filter_map(|name| name.ok().flatten().map(|name| name.to_owned()))
          .collect()
      })
      .convert("Fetch remotes failed")
  }

  #[napi]
  /// Get the information for a particular remote
  pub fn find_remote(
    &self,
    self_ref: Reference<Repository>,
    env: Env,
    name: String,
  ) -> Option<Remote> {
    Some(Remote {
      inner: self_ref
        .share_with(env, move |repo| {
          repo
            .inner
            .find_remote(&name)
            .convert(format!("Failed to get remote [{}]", &name))
        })
        .ok()?,
    })
  }

  #[napi]
  /// Add a remote with the default fetch refspec to the repository's
  /// configuration.
  pub fn remote(
    &mut self,
    env: Env,
    this: Reference<Repository>,
    name: String,
    url: String,
  ) -> Result<Remote> {
    Ok(Remote {
      inner: this.share_with(env, move |repo| {
        repo
          .inner
          .remote(&name, &url)
          .convert(format!("Failed to add remote [{}]", &name))
      })?,
    })
  }

  #[napi]
  /// Add a remote with the provided fetch refspec to the repository's
  /// configuration.
  pub fn remote_with_fetch(
    &mut self,
    env: Env,
    this: Reference<Repository>,
    name: String,
    url: String,
    refspect: String,
  ) -> Result<Remote> {
    Ok(Remote {
      inner: this.share_with(env, move |repo| {
        repo
          .inner
          .remote_with_fetch(&name, &url, &refspect)
          .convert("Failed to add remote")
      })?,
    })
  }

  #[napi]
  /// Create an anonymous remote
  ///
  /// Create a remote with the given URL and refspec in memory. You can use
  /// this when you have a URL instead of a remote's name. Note that anonymous
  /// remotes cannot be converted to persisted remotes.
  pub fn remote_anonymous(
    &self,
    env: Env,
    this: Reference<Repository>,
    url: String,
  ) -> Result<Remote> {
    Ok(Remote {
      inner: this.share_with(env, move |repo| {
        repo
          .inner
          .remote_anonymous(&url)
          .convert("Failed to create anonymous remote")
      })?,
    })
  }

  #[napi]
  /// Give a remote a new name
  ///
  /// All remote-tracking branches and configuration settings for the remote
  /// are updated.
  ///
  /// A temporary in-memory remote cannot be given a name with this method.
  ///
  /// No loaded instances of the remote with the old name will change their
  /// name or their list of refspecs.
  ///
  /// The returned array of strings is a list of the non-default refspecs
  /// which cannot be renamed and are returned for further processing by the
  /// caller.
  pub fn remote_rename(&self, name: String, new_name: String) -> Result<Vec<String>> {
    Ok(
      self
        .inner
        .remote_rename(&name, &new_name)
        .convert(format!("Failed to rename remote [{}]", &name))?
        .into_iter()
        .filter_map(|s| s.ok().flatten().map(|s| s.to_owned()))
        .collect::<Vec<_>>(),
    )
  }

  #[napi]
  /// Delete an existing persisted remote.
  ///
  /// All remote-tracking branches and configuration settings for the remote
  /// will be removed.
  pub fn remote_delete(&self, name: String) -> Result<&Self> {
    self.inner.remote_delete(&name).convert_without_message()?;
    Ok(self)
  }

  #[napi]
  /// Add a fetch refspec to the remote's configuration
  ///
  /// Add the given refspec to the fetch list in the configuration. No loaded
  pub fn remote_add_fetch(&self, name: String, refspec: String) -> Result<&Self> {
    self
      .inner
      .remote_add_fetch(&name, &refspec)
      .convert_without_message()?;
    Ok(self)
  }

  #[napi]
  /// Add a push refspec to the remote's configuration.
  ///
  /// Add the given refspec to the push list in the configuration. No
  /// loaded remote instances will be affected.
  pub fn remote_add_push(&self, name: String, refspec: String) -> Result<&Self> {
    self
      .inner
      .remote_add_push(&name, &refspec)
      .convert_without_message()?;
    Ok(self)
  }

  #[napi]
  /// Add a push refspec to the remote's configuration.
  ///
  /// Add the given refspec to the push list in the configuration. No
  /// loaded remote instances will be affected.
  pub fn remote_set_url(&self, name: String, url: String) -> Result<&Self> {
    self
      .inner
      .remote_set_url(&name, &url)
      .convert_without_message()?;
    Ok(self)
  }

  #[napi]
  /// Set the remote's URL for pushing in the configuration.
  ///
  /// Remote objects already in memory will not be affected. This assumes
  /// the common case of a single-url remote and will otherwise return an
  /// error.
  ///
  /// `None` indicates that it should be cleared.
  pub fn remote_set_pushurl(&self, name: String, url: Option<String>) -> Result<&Self> {
    self
      .inner
      .remote_set_pushurl(&name, url.as_deref())
      .convert_without_message()?;
    Ok(self)
  }

  #[napi]
  /// Lookup a reference to one of the objects in a repository.
  pub fn find_tree(&self, oid: String, self_ref: Reference<Repository>, env: Env) -> Option<Tree> {
    Some(Tree {
      inner: TreeParent::Repository(
        self_ref
          .share_with(env, |repo| {
            repo
              .inner
              .find_tree(git2::Oid::from_str(oid.as_str()).convert(format!("Invalid OID [{oid}]"))?)
              .convert(format!("Find tree from OID [{oid}] failed"))
          })
          .ok()?,
      ),
    })
  }

  #[napi]
  pub fn find_commit(
    &self,
    oid: String,
    this_ref: Reference<Repository>,
    env: Env,
  ) -> Option<Commit> {
    let commit = this_ref
      .share_with(env, |repo| {
        repo
          .inner
          .find_commit_by_prefix(&oid)
          .convert(format!("Find commit from OID [{oid}] failed"))
      })
      .ok()?;
    Some(Commit {
      inner: CommitInner::Repository(commit),
    })
  }

  #[napi]
  /// List the branches in the repository.
  ///
  /// Pass `filter` to restrict the listing to local or remote branches; omit it
  /// to list both. Branches whose names are not valid utf-8 are skipped (they
  /// cannot be re-resolved by name).
  pub fn branches(
    &self,
    this_ref: Reference<Repository>,
    env: Env,
    filter: Option<BranchType>,
  ) -> Result<Vec<Branch>> {
    // The `Branches<'repo>` iterator yields branches borrowing the repository,
    // which cannot escape into a `SharedReference`. Collect each branch's name
    // and type first, then rebuild owned `Branch`es by re-finding them.
    let mut specs: Vec<(String, git2::BranchType)> = Vec::new();
    {
      let branches = self
        .inner
        .branches(filter.map(Into::into))
        .convert("Failed to list branches")?;
      for branch in branches {
        let (branch, branch_type) = branch.convert_without_message()?;
        if let Some(name) = branch.name().convert_without_message()? {
          specs.push((name.to_owned(), branch_type));
        }
      }
    }
    let mut result = Vec::with_capacity(specs.len());
    for (name, branch_type) in specs {
      let inner = this_ref.clone(env)?.share_with(env, move |repo| {
        repo
          .inner
          .find_branch(&name, branch_type)
          .convert(format!("Find branch [{name}] failed"))
      })?;
      result.push(Branch { inner });
    }
    Ok(result)
  }

  #[napi]
  /// Lookup a branch by its name in a repository.
  ///
  /// Returns `null` when no branch with that name and type exists.
  pub fn find_branch(
    &self,
    this_ref: Reference<Repository>,
    env: Env,
    name: String,
    branch_type: BranchType,
  ) -> Result<Option<Branch>> {
    let branch_type: git2::BranchType = branch_type.into();
    // Probe first so a genuine "not found" maps to `None` while other errors
    // surface, and the `SharedReference` is only built when the branch exists.
    if let Err(err) = self.inner.find_branch(&name, branch_type) {
      if err.code() == git2::ErrorCode::NotFound {
        return Ok(None);
      }
      return Err(err).convert(format!("Find branch [{name}] failed"));
    }
    let inner = this_ref.share_with(env, move |repo| {
      repo
        .inner
        .find_branch(&name, branch_type)
        .convert(format!("Find branch [{name}] failed"))
    })?;
    Ok(Some(Branch { inner }))
  }

  #[napi]
  /// Create a new branch pointing at a target commit.
  ///
  /// A new direct reference will be created pointing to this target commit. If
  /// `force` is true and a branch already exists with the given name, it will
  /// be replaced.
  pub fn branch(
    &self,
    this_ref: Reference<Repository>,
    env: Env,
    branch_name: String,
    target: &Commit,
    force: bool,
  ) -> Result<Branch> {
    // Create the branch ref; the returned borrowed branch is dropped, then the
    // owned `Branch` is rebuilt by re-finding it inside `share_with`.
    self
      .inner
      .branch(&branch_name, &target.inner, force)
      .convert(format!("Failed to create branch [{branch_name}]"))?;
    let inner = this_ref.share_with(env, move |repo| {
      repo
        .inner
        .find_branch(&branch_name, git2::BranchType::Local)
        .convert(format!("Find branch [{branch_name}] failed"))
    })?;
    Ok(Branch { inner })
  }

  #[napi]
  /// Check out the tree pointed to by `treeish` (a commit, tag or tree object),
  /// updating the working directory to match.
  ///
  /// This does NOT update HEAD; pair it with `set_head` to switch branches.
  /// The checkout is **safe** by default — pass `options.force = true` to
  /// overwrite local modifications.
  pub fn checkout_tree(&self, treeish: &GitObject, options: Option<CheckoutOptions>) -> Result<()> {
    let mut builder = build_checkout_builder(options);
    self
      .inner
      .checkout_tree(&treeish.inner, Some(&mut builder))
      .convert_without_message()
  }

  #[napi]
  /// Update files in the index and the working tree to match the content of
  /// the tree pointed at by HEAD.
  ///
  /// The checkout is **safe** by default — pass `options.force = true` to
  /// overwrite local modifications.
  pub fn checkout_head(&self, options: Option<CheckoutOptions>) -> Result<()> {
    let mut builder = build_checkout_builder(options);
    self
      .inner
      .checkout_head(Some(&mut builder))
      .convert_without_message()
  }

  #[napi]
  /// Update files in the working tree to match the content of the repository's
  /// index.
  ///
  /// The checkout is **safe** by default — pass `options.force = true` to
  /// overwrite local modifications.
  pub fn checkout_index(&self, options: Option<CheckoutOptions>) -> Result<()> {
    let mut builder = build_checkout_builder(options);
    self
      .inner
      .checkout_index(None, Some(&mut builder))
      .convert_without_message()
  }

  #[napi]
  /// Make HEAD point to the reference named `refname`.
  ///
  /// If `refname` names an existing branch, HEAD becomes a symbolic reference
  /// to that branch; otherwise it points to a not-yet-existing branch. This
  /// does not touch the working directory — checkout separately.
  pub fn set_head(&self, refname: String) -> Result<()> {
    self.inner.set_head(&refname).convert_without_message()
  }

  #[napi]
  /// Make HEAD point directly at the commit with the given OID, detaching it
  /// from any branch.
  pub fn set_head_detached(&self, oid: String) -> Result<()> {
    let oid = git2::Oid::from_str(&oid).convert(format!("Invalid OID [{oid}]"))?;
    self.inner.set_head_detached(oid).convert_without_message()
  }

  #[napi]
  /// Create a new direct reference named `name` pointing at the object `oid`.
  ///
  /// If `force` is true and a reference already exists with the given name, it
  /// will be overwritten; otherwise the call fails. `log_message` is recorded
  /// in the reflog.
  pub fn reference(
    &self,
    this_ref: Reference<Repository>,
    env: Env,
    name: String,
    oid: String,
    force: bool,
    log_message: String,
  ) -> Result<reference::Reference> {
    Ok(reference::Reference {
      inner: this_ref.share_with(env, move |repo| {
        let oid = git2::Oid::from_str(&oid).convert(format!("Invalid OID [{oid}]"))?;
        repo
          .inner
          .reference(&name, oid, force, &log_message)
          .convert(format!("Failed to create reference [{name}]"))
      })?,
    })
  }

  #[napi]
  /// Create a new symbolic reference named `name` pointing at the reference
  /// named `target` (e.g. `refs/heads/main`).
  ///
  /// If `force` is true and a reference already exists with the given name, it
  /// will be overwritten; otherwise the call fails. `log_message` is recorded
  /// in the reflog.
  pub fn reference_symbolic(
    &self,
    this_ref: Reference<Repository>,
    env: Env,
    name: String,
    target: String,
    force: bool,
    log_message: String,
  ) -> Result<reference::Reference> {
    Ok(reference::Reference {
      inner: this_ref.share_with(env, move |repo| {
        repo
          .inner
          .reference_symbolic(&name, &target, force, &log_message)
          .convert(format!("Failed to create symbolic reference [{name}]"))
      })?,
    })
  }

  #[napi]
  /// Create a new tag in the repository from an object
  ///
  /// A new reference will also be created pointing to this tag object. If
  /// `force` is true and a reference already exists with the given name,
  /// it'll be replaced.
  ///
  /// The message will not be cleaned up.
  ///
  /// The tag name will be checked for validity. You must avoid the characters
  /// '~', '^', ':', ' \ ', '?', '[', and '*', and the sequences ".." and " @
  /// {" which have special meaning to revparse.
  pub fn tag(
    &self,
    name: String,
    target: &GitObject,
    tagger: &Signature,
    message: String,
    force: bool,
  ) -> Result<String> {
    self
      .inner
      .tag(&name, &target.inner, &tagger.inner, &message, force)
      .map(|o| o.to_string())
      .convert("Failed to create tag")
  }

  #[napi]
  /// Create a new tag in the repository from an object without creating a reference.
  ///
  /// The message will not be cleaned up.
  ///
  /// The tag name will be checked for validity. You must avoid the characters
  /// '~', '^', ':', ' \ ', '?', '[', and '*', and the sequences ".." and " @
  /// {" which have special meaning to revparse.
  pub fn tag_annotation_create(
    &self,
    name: String,
    target: &GitObject,
    tagger: &Signature,
    message: String,
  ) -> Result<String> {
    self
      .inner
      .tag_annotation_create(&name, &target.inner, &tagger.inner, &message)
      .map(|o| o.to_string())
      .convert("Failed to create tag annotation")
  }

  #[napi]
  /// Create a new lightweight tag pointing at a target object
  ///
  /// A new direct reference will be created pointing to this target object.
  /// If force is true and a reference already exists with the given name,
  /// it'll be replaced.
  pub fn tag_lightweight(&self, name: String, target: &GitObject, force: bool) -> Result<String> {
    self
      .inner
      .tag_lightweight(&name, &target.inner, force)
      .map(|o| o.to_string())
      .convert("Failed to create lightweight tag")
  }

  #[napi]
  /// Lookup a tag object from the repository.
  pub fn find_tag(&self, env: Env, this: Reference<Repository>, oid: String) -> Result<Tag> {
    Ok(Tag {
      inner: this.share_with(env, |repo| {
        repo
          .inner
          .find_tag(git2::Oid::from_str(oid.as_str()).convert(format!("Invalid OID [{oid}]"))?)
          .convert(format!("Find tag from OID [{oid}] failed"))
      })?,
    })
  }

  #[napi]
  /// Lookup a tag object by prefix hash from the repository.
  pub fn find_tag_by_prefix(
    &self,
    env: Env,
    this: Reference<Repository>,
    prefix_hash: String,
  ) -> Result<Tag> {
    Ok(Tag {
      inner: this.share_with(env, |repo| {
        repo
          .inner
          .find_tag_by_prefix(&prefix_hash)
          .convert(format!("Find tag from OID [{prefix_hash}] failed"))
      })?,
    })
  }

  #[napi]
  /// Delete an existing tag reference.
  ///
  /// The tag name will be checked for validity, see `tag` for some rules
  /// about valid names.
  pub fn tag_delete(&self, name: String) -> Result<()> {
    self.inner.tag_delete(&name).convert_without_message()?;
    Ok(())
  }

  #[napi]
  /// Get a list with all the tags in the repository.
  ///
  /// An optional fnmatch pattern can also be specified.
  pub fn tag_names(&self, pattern: Option<String>) -> Result<Vec<String>> {
    self
      .inner
      .tag_names(pattern.as_deref())
      .convert("Failed to get tag names")
      .map(|tags| {
        tags
          .into_iter()
          .filter_map(|s| s.ok().flatten().map(|s| s.to_owned()))
          .collect()
      })
  }

  #[napi]
  /// iterate over all tags calling `cb` on each.
  /// the callback is provided the tag id and name
  pub fn tag_foreach(&self, cb: Function<(String, Buffer), bool>) -> Result<()> {
    self
      .inner
      .tag_foreach(|oid, name| {
        let oid = oid.to_string();
        let name = name.to_vec();
        cb.call((oid, name.into())).unwrap_or(false)
      })
      .convert_without_message()
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
  pub fn tree_entry_to_object(
    &self,
    tree_entry: &TreeEntry,
    this_ref: Reference<Repository>,
    env: Env,
  ) -> Result<GitObject> {
    Ok(GitObject {
      inner: ObjectParent::Repository(this_ref.share_with(env, |repo| {
        tree_entry
          .inner
          .to_object(&repo.inner)
          .convert_without_message()
      })?),
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
  ///
  /// `parents` is an optional list of parent commit OID hex strings. When it
  /// is `None` or empty a parent-less root commit is created; otherwise each
  /// OID is resolved to a commit and used as a parent (the first parent must
  /// be the current tip of `update_ref`).
  pub fn commit(
    &self,
    update_ref: Option<String>,
    author: &Signature,
    committer: &Signature,
    message: String,
    tree: &Tree,
    parents: Option<Vec<String>>,
  ) -> Result<String> {
    let parent_commits = parents
      .unwrap_or_default()
      .into_iter()
      .map(|oid| {
        let oid = git2::Oid::from_str(&oid).convert(format!("Invalid OID [{oid}]"))?;
        self
          .inner
          .find_commit(oid)
          .convert(format!("Find commit from OID [{oid}] failed"))
      })
      .collect::<Result<Vec<git2::Commit>>>()?;
    let parent_refs = parent_commits.iter().collect::<Vec<&git2::Commit>>();
    self
      .inner
      .commit(
        update_ref.as_deref(),
        author.as_ref(),
        committer.as_ref(),
        message.as_str(),
        tree.as_ref(),
        &parent_refs,
      )
      .convert_without_message()
      .map(|oid| oid.to_string())
  }

  #[napi]
  /// Get the index (staging area) file for this repository.
  ///
  /// If a custom index has not been set, the default index for the repository
  /// will be returned (the one at `.git/index`).
  pub fn index(&self) -> Result<Index> {
    Ok(Index {
      inner: self.inner.index().convert_without_message()?,
    })
  }

  #[napi]
  /// Write an in-memory buffer to the object database as a blob and return its
  /// OID hex string.
  pub fn blob(&self, data: Uint8Array) -> Result<String> {
    self
      .inner
      .blob(&data)
      .map(|oid| oid.to_string())
      .convert_without_message()
  }

  #[napi]
  /// Read a file from the filesystem and write its content to the object
  /// database as a blob, returning its OID hex string.
  pub fn blob_path(&self, path: String) -> Result<String> {
    self
      .inner
      .blob_path(Path::new(&path))
      .map(|oid| oid.to_string())
      .convert_without_message()
  }

  #[napi]
  /// Create a revwalk that can be used to traverse the commit graph.
  pub fn rev_walk(&self, this_ref: Reference<Repository>, env: Env) -> Result<RevWalk> {
    Ok(RevWalk {
      inner: this_ref.share_with(env, |repo| repo.inner.revwalk().convert_without_message())?,
    })
  }

  #[napi]
  pub fn get_file_latest_modified_date(&self, filepath: String) -> Result<i64> {
    get_file_modification(&self.inner, &filepath)
      .convert_without_message()
      .and_then(|value| {
        value
          .map(|m| m.timestamp)
          .expect_not_null(format!("Failed to get commit for [{filepath}]"))
      })
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
        repo: RwLock::new(self_ref),
        filepath,
      },
      signal,
    ))
  }

  #[napi]
  /// Last commit that modified `filepath`, with author/committer identity.
  /// Returns `null` when no commit in history touched the path.
  pub fn get_file_latest_modification(&self, filepath: String) -> Result<Option<FileModification>> {
    get_file_modification(&self.inner, &filepath).convert_without_message()
  }

  #[napi]
  pub fn get_file_latest_modification_async(
    &self,
    self_ref: Reference<Repository>,
    filepath: String,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<GitModificationTask>> {
    Ok(AsyncTask::with_optional_signal(
      GitModificationTask {
        repo: RwLock::new(self_ref),
        filepath,
      },
      signal,
    ))
  }

  #[napi]
  /// Resolve the last commit that modified each of `filepaths` in a single
  /// history walk. Every input path is a key; never-committed paths map to `null`.
  pub fn get_files_latest_modification(
    &self,
    filepaths: Vec<String>,
  ) -> Result<HashMap<String, Option<FileModification>>> {
    get_files_modification(&self.inner, &filepaths).convert_without_message()
  }

  #[napi]
  pub fn get_files_latest_modification_async(
    &self,
    self_ref: Reference<Repository>,
    filepaths: Vec<String>,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<GitBulkModificationTask>> {
    Ok(AsyncTask::with_optional_signal(
      GitBulkModificationTask {
        repo: RwLock::new(self_ref),
        filepaths,
      },
      signal,
    ))
  }

  #[napi]
  /// List the working-tree and index status of files in the repository.
  ///
  /// Mirrors `git status`. By default untracked files are included and ignored
  /// files are not; pass `options` to tune the scan. Each returned `FileStatus`
  /// decodes the `git2::Status` flags into booleans plus the raw `bits`.
  pub fn statuses(&self, options: Option<StatusOptions>) -> Result<Vec<FileStatus>> {
    collect_statuses(&self.inner, options)
  }

  #[napi]
  /// Get the status of a single file by its workdir-relative path.
  ///
  /// This is more efficient than scanning the whole tree when only one path is
  /// of interest. Errors (e.g. an ambiguous path) surface as a napi error.
  pub fn status_file(&self, path: String) -> Result<FileStatus> {
    let status = self
      .inner
      .status_file(Path::new(&path))
      .convert_without_message()?;
    Ok(status_from_bits(status, Some(path)))
  }

  #[napi]
  /// Asynchronous variant of `statuses`, computed off the main thread.
  pub fn statuses_async(
    &self,
    self_ref: Reference<Repository>,
    options: Option<StatusOptions>,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<GitStatusTask>> {
    Ok(AsyncTask::with_optional_signal(
      GitStatusTask {
        repo: RwLock::new(self_ref),
        options,
      },
      signal,
    ))
  }

  #[napi]
  /// Compute the blame for `path`: who last changed each line, as an ordered
  /// list of hunks (contiguous runs of lines sharing one final commit).
  ///
  /// `path` is workdir-relative. Pass `options` to restrict the line/commit
  /// range or enable copy tracking. Each `BlameHunk` is eagerly materialized
  /// so it outlives the underlying libgit2 blame.
  pub fn blame_file(&self, path: String, options: Option<BlameOptions>) -> Result<Vec<BlameHunk>> {
    collect_blame(&self.inner, &path, options)
  }

  #[napi]
  /// Blame `path` and return only the hunk covering `line_no` (1-based), or
  /// `null` when the line is out of range.
  pub fn blame_line(
    &self,
    path: String,
    line_no: u32,
    options: Option<BlameOptions>,
  ) -> Result<Option<BlameHunk>> {
    blame_single_line(&self.inner, &path, line_no, options)
  }

  #[napi]
  /// Asynchronous variant of `blame_file`, computed off the main thread.
  pub fn blame_file_async(
    &self,
    self_ref: Reference<Repository>,
    path: String,
    options: Option<BlameOptions>,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<GitBlameTask>> {
    Ok(AsyncTask::with_optional_signal(
      GitBlameTask {
        repo: RwLock::new(self_ref),
        filepath: path,
        options,
      },
      signal,
    ))
  }

  #[napi]
  pub fn get_file_created_date(&self, filepath: String) -> Result<i64> {
    get_file_created_date(&self.inner, &filepath)
      .convert_without_message()
      .and_then(|value| {
        value.expect_not_null(format!("Failed to get created date for [{filepath}]"))
      })
  }

  #[napi]
  pub fn get_file_created_date_async(
    &self,
    self_ref: Reference<Repository>,
    filepath: String,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<GitCreatedDateTask>> {
    Ok(AsyncTask::with_optional_signal(
      GitCreatedDateTask {
        repo: RwLock::new(self_ref),
        filepath,
      },
      signal,
    ))
  }
}

/// Run a status scan and eagerly materialize the borrowed `Statuses<'repo>`
/// into owned `FileStatus` values so nothing referencing the repository escapes.
fn collect_statuses(
  repo: &git2::Repository,
  options: Option<StatusOptions>,
) -> Result<Vec<FileStatus>> {
  let mut opts = build_status_opts(options);
  let statuses = repo.statuses(Some(&mut opts)).convert_without_message()?;
  Ok(
    statuses
      .iter()
      .map(|entry| {
        let path = entry.path().ok().map(|p| p.to_owned());
        status_from_bits(entry.status(), path)
      })
      .collect(),
  )
}

fn get_file_created_date(
  repo: &git2::Repository,
  filepath: &str,
) -> std::result::Result<Option<i64>, git2::Error> {
  // TODO: Add rename detection support using git2::DiffFindOptions for full `git log --follow` semantics
  let mut rev_walk = repo.revwalk()?;
  rev_walk.push_head()?;
  // Sort::TIME | Sort::TOPOLOGICAL (newest-first): the walk overwrites the
  // recorded time for each commit that still contains the file, so the last
  // one visited -- the oldest containing commit -- is the creation commit.
  rev_walk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL)?;
  let path = PathBuf::from(filepath);

  let mut earliest_commit_time: Option<i64> = None;

  // Traverse all commits to find the earliest one that contains the file
  for oid in rev_walk.by_ref().filter_map(|oid| oid.ok()) {
    if let Ok(commit) = repo.find_commit(oid)
      && let Ok(tree) = commit.tree()
    {
      // Check if the file exists in this commit's tree
      if tree.get_path(&path).is_ok() {
        earliest_commit_time = Some(commit.time().seconds() * 1000);
      }
    }
  }

  Ok(earliest_commit_time)
}
