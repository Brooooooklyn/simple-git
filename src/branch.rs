use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::error::IntoNapiError;

#[napi]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// An enumeration for the possible types of branches.
pub enum BranchType {
  /// A local branch not on a remote.
  Local,
  /// A branch for a remote.
  Remote,
}

impl From<BranchType> for git2::BranchType {
  fn from(value: BranchType) -> Self {
    match value {
      BranchType::Local => git2::BranchType::Local,
      BranchType::Remote => git2::BranchType::Remote,
    }
  }
}

impl From<git2::BranchType> for BranchType {
  fn from(value: git2::BranchType) -> Self {
    match value {
      git2::BranchType::Local => BranchType::Local,
      git2::BranchType::Remote => BranchType::Remote,
    }
  }
}

#[napi]
/// A git branch.
///
/// A branch is a thin wrapper around an underlying reference; the full
/// reference name is available via `reference_name`.
pub struct Branch {
  pub(crate) inner:
    napi::bindgen_prelude::SharedReference<crate::repo::Repository, git2::Branch<'static>>,
}

#[napi]
impl Branch {
  #[napi]
  /// Return the name of the given local or remote branch.
  ///
  /// Returns `None` if the name is not valid utf-8.
  pub fn name(&self) -> Result<Option<String>> {
    Ok(
      self
        .inner
        .name()
        .convert_without_message()?
        .map(|name| name.to_owned()),
    )
  }

  #[napi]
  /// Determine if the current local branch is pointed at by HEAD.
  pub fn is_head(&self) -> bool {
    self.inner.is_head()
  }

  #[napi]
  /// Get the full name of the reference backing this branch
  /// (e.g. `refs/heads/main`).
  ///
  /// Returns `None` if the reference name is not valid utf-8.
  pub fn reference_name(&self) -> Option<String> {
    self.inner.get().name().ok().map(|name| name.to_owned())
  }

  #[napi]
  /// Delete an existing branch reference.
  pub fn delete(&mut self) -> Result<()> {
    self.inner.delete().convert_without_message()
  }

  #[napi]
  /// Return the reference supporting the remote tracking branch, given a local
  /// branch reference.
  ///
  /// Returns `None` when the branch has no configured upstream.
  pub fn upstream(&self, env: Env) -> Result<Option<Branch>> {
    match self
      .inner
      .clone(env)?
      .share_with(env, |branch| branch.upstream().convert_without_message())
    {
      Ok(inner) => Ok(Some(Branch { inner })),
      Err(_) => Ok(None),
    }
  }

  #[napi]
  /// Return the reference backing this branch as a live `Reference`.
  ///
  /// Branches are direct references, so the resolved direct reference is
  /// returned (e.g. `refs/heads/main`).
  pub fn get(&self, env: Env) -> Result<crate::reference::Reference> {
    let inner = self.inner.clone(env)?.share_with(env, |branch| {
      branch.get().resolve().convert_without_message()
    })?;
    Ok(crate::reference::Reference { inner })
  }
}
