use std::ops::Deref;

use napi::bindgen_prelude::{Buffer, Env, Error, Generator, Reference, Result, SharedReference};
use napi_derive::napi;

pub(crate) enum TreeParent {
  Repository(SharedReference<crate::repo::Repository, git2::Tree<'static>>),
  Reference(SharedReference<crate::reference::Reference, git2::Tree<'static>>),
  Commit(SharedReference<crate::commit::Commit, git2::Tree<'static>>),
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
      TreeParent::Commit(parent) => parent,
    }
  }

  #[napi]
  /// Get the id (SHA1) of a repository object
  pub fn id(&self) -> String {
    self.inner().id().to_string()
  }

  #[napi]
  /// Get the number of entries listed in a tree.
  pub fn len(&self) -> u64 {
    self.inner().len() as u64
  }

  #[napi]
  /// Return `true` if there is not entry
  pub fn is_empty(&self) -> bool {
    self.inner().is_empty()
  }

  #[napi]
  /// Returns an iterator over the entries in this tree.
  pub fn iter(&self, this_ref: Reference<Tree>, env: Env) -> Result<TreeIter> {
    Ok(TreeIter {
      inner: this_ref.share_with(env, |tree| Ok(tree.inner().iter()))?,
    })
  }
}

impl<'a> AsRef<git2::Tree<'a>> for Tree {
  fn as_ref(&self) -> &git2::Tree<'a> {
    match self.inner {
      TreeParent::Repository(ref parent) => parent.deref(),
      TreeParent::Reference(ref parent) => parent.deref(),
      TreeParent::Commit(ref parent) => parent.deref(),
    }
  }
}

#[napi(iterator)]
pub struct TreeIter {
  pub(crate) inner: SharedReference<Tree, git2::TreeIter<'static>>,
}

#[napi]
impl Generator for TreeIter {
  type Yield = TreeEntry;
  type Return = ();
  type Next = ();

  fn next(&mut self, _value: Option<()>) -> Option<Self::Yield> {
    self.inner.next().map(|e| TreeEntry { inner: e })
  }
}

#[napi]
pub struct TreeEntry {
  pub(crate) inner: git2::TreeEntry<'static>,
}

#[napi]
impl TreeEntry {
  #[napi]
  /// Get the id of the object pointed by the entry
  pub fn id(&self) -> String {
    self.inner.id().to_string()
  }

  #[napi]
  /// Get the name of a tree entry
  pub fn name(&self) -> Result<&str> {
    self
      .inner
      .name()
      .ok_or_else(|| Error::from_reason("Invalid utf-8"))
  }

  #[napi]
  /// Get the filename of a tree entry
  pub fn name_bytes(&self) -> Buffer {
    self.inner.name_bytes().to_vec().into()
  }
}
