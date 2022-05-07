use napi::bindgen_prelude::*;
use napi_derive::napi;

pub(crate) enum TreeParent {
  Repository(SharedReference<crate::repo::Repository, git2::Tree<'static>>),
  Reference(SharedReference<crate::reference::Reference, git2::Tree<'static>>),
}

#[napi]
pub struct Tree {
  pub(crate) inner: TreeParent,
}

#[napi]
impl Tree {
  pub(crate) fn inner(&self) -> &git2::Tree {
    match &self.inner {
      TreeParent::Repository(parent) => parent,
      TreeParent::Reference(parent) => parent,
    }
  }

  #[napi]
  /// Get the id (SHA1) of a repository object
  pub fn id(&self) -> String {
    self.inner().id().to_string()
  }
}
