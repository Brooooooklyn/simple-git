use std::{mem, path::Path};

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{error::IntoNapiError, remote::FetchOptions, repo::Repository};

#[napi]
pub struct RepoBuilder {
  builder: git2::build::RepoBuilder<'static>,
}

#[napi]
pub enum CloneLocal {
  /// Auto-detect (default)
  ///
  /// Here libgit2 will bypass the git-aware transport for local paths, but
  /// use a normal fetch for `file://` URLs.
  Auto,

  /// Bypass the git-aware transport even for `file://` URLs.
  Local,

  /// Never bypass the git-aware transport
  None,

  /// Bypass the git-aware transport, but don't try to use hardlinks.
  NoLinks,
}

impl From<CloneLocal> for git2::build::CloneLocal {
  fn from(clone_local: CloneLocal) -> Self {
    match clone_local {
      CloneLocal::Auto => git2::build::CloneLocal::Auto,
      CloneLocal::Local => git2::build::CloneLocal::Local,
      CloneLocal::None => git2::build::CloneLocal::None,
      CloneLocal::NoLinks => git2::build::CloneLocal::NoLinks,
    }
  }
}

#[napi]
/// A builder struct which is used to build configuration for cloning a new git
/// repository.
///
/// # Example
///
/// Cloning using SSH:
///
/// ```rust
/// use git2::{Cred, Error, RemoteCallbacks};
/// use std::env;
/// use std::path::Path;
///
///   // Prepare callbacks.
///   let mut callbacks = RemoteCallbacks::new();
///   callbacks.credentials(|_url, username_from_url, _allowed_types| {
///     Cred::ssh_key(
///       username_from_url.unwrap(),
///       None,
///       Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
///       None,
///     )
///   });
///
///   // Prepare fetch options.
///   let mut fo = git2::FetchOptions::new();
///   fo.remote_callbacks(callbacks);
///
///   // Prepare builder.
///   let mut builder = git2::build::RepoBuilder::new();
///   builder.fetch_options(fo);
///
///   // Clone the project.
///   builder.clone(
///     "git@github.com:rust-lang/git2-rs.git",
///     Path::new("/tmp/git2-rs"),
///   );
/// ```
impl RepoBuilder {
  #[napi(constructor)]
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    Self {
      builder: Default::default(),
    }
  }

  #[napi]
  /// Indicate whether the repository will be cloned as a bare repository or
  /// not.
  pub fn bare(&mut self, bare: bool) -> &Self {
    self.builder.bare(bare);
    self
  }

  #[napi]
  /// Specify the name of the branch to check out after the clone.
  ///
  /// If not specified, the remote's default branch will be used.
  pub fn branch(&mut self, branch: String) -> &Self {
    self.builder.branch(&branch);
    self
  }

  #[napi]
  /// Configures options for bypassing the git-aware transport on clone.
  ///
  /// Bypassing it means that instead of a fetch libgit2 will copy the object
  /// database directory instead of figuring out what it needs, which is
  /// faster. If possible, it will hardlink the files to save space.
  pub fn clone_local(&mut self, clone_local: CloneLocal) -> &Self {
    self.builder.clone_local(clone_local.into());
    self
  }

  #[napi]
  /// Options which control the fetch, including callbacks.
  ///
  /// The callbacks are used for reporting fetch progress, and for acquiring
  /// credentials in the event they are needed.
  pub fn fetch_options(&mut self, fetch_options: &mut FetchOptions) -> Result<&Self> {
    if fetch_options.used {
      return Err(Error::new(
        Status::GenericFailure,
        "FetchOptions has been used, please create a new one",
      ));
    }
    let mut opt = git2::FetchOptions::default();
    mem::swap(&mut fetch_options.inner, &mut opt);
    fetch_options.used = true;
    self.builder.fetch_options(opt);
    Ok(self)
  }

  #[napi]
  pub fn clone(&mut self, url: String, path: String) -> Result<Repository> {
    Ok(Repository {
      inner: self
        .builder
        .clone(&url, &Path::new(&path))
        .convert("Clone failed")?,
    })
  }
}
