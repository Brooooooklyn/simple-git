use std::path::PathBuf;
use std::sync::RwLock;

use napi::{JsString, bindgen_prelude::*};
use napi_derive::napi;
use once_cell::sync::Lazy;

use crate::commit::{Commit, CommitInner};
use crate::diff::Diff;
use crate::error::{IntoNapiError, NotNullError};
use crate::object::{GitObject, ObjectParent};
use crate::reference;
use crate::remote::Remote;
use crate::rev_walk::RevWalk;
use crate::signature::Signature;
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
    if git2::Config::find_global().is_err() {
      if let Some(mut git_config_dir) = dirs::home_dir() {
        git_config_dir.push(".gitconfig");
        std::fs::write(&git_config_dir, "").map_err(|err| {
          Error::new(
            Status::GenericFailure,
            format!("Initialize {git_config_dir:?} failed {err}"),
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
  repo: RwLock<napi::bindgen_prelude::Reference<Repository>>,
  filepath: String,
}

unsafe impl Send for GitDateTask {}

#[napi]
impl Task for GitDateTask {
  type Output = i64;
  type JsValue = i64;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    get_file_modified_date(
      &self
        .repo
        .read()
        .map_err(|err| napi::Error::new(Status::GenericFailure, format!("{err}")))?
        .inner,
      &self.filepath,
    )
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
        .flatten()
        .map(|s| s.to_owned())
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
          .filter_map(|s| s.map(|s| s.to_owned()))
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
  /// Create a revwalk that can be used to traverse the commit graph.
  pub fn rev_walk(&self, this_ref: Reference<Repository>, env: Env) -> Result<RevWalk> {
    Ok(RevWalk {
      inner: this_ref.share_with(env, |repo| repo.inner.revwalk().convert_without_message())?,
    })
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
        repo: RwLock::new(self_ref),
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
