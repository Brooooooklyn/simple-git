use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use napi::bindgen_prelude::{Buffer, SharedReference};
use napi_derive::napi;

use crate::ensure_alive;
use crate::object::GitObject;

pub(crate) enum BlobParent {
  GitObject(SharedReference<GitObject, git2::Blob<'static>>),
}

impl Deref for BlobParent {
  type Target = git2::Blob<'static>;

  fn deref(&self) -> &git2::Blob<'static> {
    match self {
      BlobParent::GitObject(parent) => parent.deref(),
    }
  }
}

#[napi]
pub struct Blob {
  pub(crate) inner: BlobParent,
  /// Liveness flag shared with the owning `Repository` (see `Repository::alive`).
  /// Guards every method that derefs the underlying `git2::Blob`.
  pub(crate) alive: Arc<AtomicBool>,
}

#[napi]
impl Blob {
  #[napi]
  /// Get the id (SHA1) of a repository blob
  pub fn id(&self) -> crate::Result<String> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.id().to_string())
  }

  #[napi]
  /// Determine if the blob content is most certainly binary or not.
  pub fn is_binary(&self) -> crate::Result<bool> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.is_binary())
  }

  #[napi]
  /// Get the content of this blob.
  pub fn content(&self) -> crate::Result<Buffer> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.content().to_vec().into())
  }

  #[napi]
  /// Get the size in bytes of the contents of this blob.
  pub fn size(&self) -> crate::Result<i64> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.size() as i64)
  }
}
