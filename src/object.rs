use std::ops::Deref;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{error::IntoNapiError, repo::Repository};

#[napi]
pub enum ObjectType {
  /// Any kind of git object
  Any,
  /// An object which corresponds to a git commit
  Commit,
  /// An object which corresponds to a git tree
  Tree,
  /// An object which corresponds to a git blob
  Blob,
  /// An object which corresponds to a git tag
  Tag,
}

impl From<git2::ObjectType> for ObjectType {
  fn from(value: git2::ObjectType) -> Self {
    match value {
      git2::ObjectType::Any => ObjectType::Any,
      git2::ObjectType::Commit => ObjectType::Commit,
      git2::ObjectType::Tree => ObjectType::Tree,
      git2::ObjectType::Blob => ObjectType::Blob,
      git2::ObjectType::Tag => ObjectType::Tag,
    }
  }
}

impl From<ObjectType> for git2::ObjectType {
  fn from(value: ObjectType) -> Self {
    match value {
      ObjectType::Any => git2::ObjectType::Any,
      ObjectType::Commit => git2::ObjectType::Commit,
      ObjectType::Tree => git2::ObjectType::Tree,
      ObjectType::Blob => git2::ObjectType::Blob,
      ObjectType::Tag => git2::ObjectType::Tag,
    }
  }
}

pub(crate) enum ObjectParent {
  Repository(SharedReference<Repository, git2::Object<'static>>),
  Object(SharedReference<GitObject, git2::Object<'static>>),
}

impl Deref for ObjectParent {
  type Target = git2::Object<'static>;

  fn deref(&self) -> &git2::Object<'static> {
    match self {
      ObjectParent::Repository(parent) => parent.deref(),
      ObjectParent::Object(parent) => parent.deref(),
    }
  }
}

#[napi]
pub struct GitObject {
  pub(crate) inner: ObjectParent,
}

#[napi]
impl GitObject {
  #[napi]
  /// Get the id (SHA1) of a repository object
  pub fn id(&self) -> String {
    self.inner.id().to_string()
  }

  #[napi]
  /// Get the type of the object.
  pub fn kind(&self) -> Option<ObjectType> {
    self.inner.kind().map(|k| k.into())
  }

  #[napi]
  pub fn peel(
    &self,
    kind: ObjectType,
    this_ref: Reference<GitObject>,
    env: Env,
  ) -> Result<GitObject> {
    Ok(GitObject {
      inner: ObjectParent::Object(this_ref.share_with(env, |o| {
        o.inner.peel(kind.into()).convert("Peel object failed")
      })?),
    })
  }
}
