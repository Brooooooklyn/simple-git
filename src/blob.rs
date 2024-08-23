use std::ops::Deref;

use napi::bindgen_prelude::{SharedReference, Uint8Array};
use napi_derive::napi;

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
}

#[napi]
impl Blob {
  #[napi]
  /// Get the id (SHA1) of a repository blob
  pub fn id(&self) -> String {
    self.inner.id().to_string()
  }

  #[napi]
  /// Determine if the blob content is most certainly binary or not.
  pub fn is_binary(&self) -> bool {
    self.inner.is_binary()
  }

  #[napi]
  /// Get the content of this blob.
  pub fn content(&self) -> Uint8Array {
    self.inner.content().to_vec().into()
  }

  #[napi]
  /// Get the size in bytes of the contents of this blob.
  pub fn size(&self) -> u64 {
    self.inner.size() as u64
  }
}
