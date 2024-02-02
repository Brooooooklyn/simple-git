use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{error::IntoNapiError, object::GitObject};

#[napi]
pub struct Tag {
  pub(crate) inner: SharedReference<crate::repo::Repository, git2::Tag<'static>>,
}

#[napi]
impl Tag {
  #[napi]
  /// Determine whether a tag name is valid, meaning that (when prefixed with refs/tags/) that
  /// it is a valid reference name, and that any additional tag name restrictions are imposed
  /// (eg, it cannot start with a -).
  pub fn is_valid_name(name: String) -> bool {
    git2::Tag::is_valid_name(&name)
  }

  #[napi]
  /// Get the id (SHA1) of a repository object
  pub fn id(&self) -> String {
    self.inner.id().to_string()
  }

  #[napi]
  /// Get the message of a tag
  ///
  /// Returns None if there is no message or if it is not valid utf8
  pub fn message(&self) -> Option<String> {
    self.inner.message().map(|s| s.to_string())
  }

  #[napi]
  /// Get the message of a tag
  ///
  /// Returns None if there is no message
  pub fn message_bytes(&self) -> Option<Buffer> {
    self.inner.message_bytes().map(|s| s.to_vec().into())
  }

  #[napi]
  /// Get the name of a tag
  ///
  /// Returns None if it is not valid utf8
  pub fn name(&self) -> Option<String> {
    self.inner.name().map(|s| s.to_string())
  }

  #[napi]
  /// Get the name of a tag
  pub fn name_bytes(&self) -> Buffer {
    self.inner.name_bytes().to_vec().into()
  }

  #[napi]
  /// Recursively peel a tag until a non tag git_object is found
  pub fn peel(&self) -> Result<GitObject> {
    let obj = self.inner.peel().convert("Peel tag failed")?;
    Ok(crate::object::GitObject {
      inner: crate::object::ObjectParent::Object(obj),
    })
  }
}
