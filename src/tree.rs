use std::ops::Deref;
use std::path::Path;

use napi::bindgen_prelude::{
  Env, Error, Generator, Reference, Result, SharedReference, Uint8Array,
};
use napi_derive::napi;

use crate::{
  error::IntoNapiError,
  object::{GitObject, ObjectParent},
  repo::Repository,
};

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

  #[napi]
  /// Lookup a tree entry by SHA value
  pub fn get_id(&self, this_ref: Reference<Tree>, env: Env, id: String) -> Option<TreeEntry> {
    let reference = this_ref
      .share_with(env, |tree| {
        if let Some(entry) = tree
          .inner()
          .get_id(git2::Oid::from_str(&id).convert_without_message()?)
        {
          Ok(entry)
        } else {
          Err(Error::new(napi::Status::InvalidArg, "Tree entry not found"))
        }
      })
      .ok()?;
    Some(TreeEntry {
      inner: TreeEntryInner::Ref(reference),
    })
  }

  #[napi]
  /// Lookup a tree entry by its position in the tree
  pub fn get(&self, this_ref: Reference<Tree>, env: Env, index: u32) -> Option<TreeEntry> {
    let reference = this_ref
      .share_with(env, |tree| {
        if let Some(entry) = tree.inner().get(index as usize) {
          Ok(entry)
        } else {
          Err(Error::new(napi::Status::InvalidArg, "Tree entry not found"))
        }
      })
      .ok()?;
    Some(TreeEntry {
      inner: TreeEntryInner::Ref(reference),
    })
  }

  #[napi]
  /// Lookup a tree entry by its filename
  pub fn get_name(&self, this_ref: Reference<Tree>, env: Env, name: String) -> Option<TreeEntry> {
    let reference = this_ref
      .share_with(env, |tree| {
        if let Some(entry) = tree.inner().get_name(&name) {
          Ok(entry)
        } else {
          Err(Error::new(napi::Status::InvalidArg, "Tree entry not found"))
        }
      })
      .ok()?;
    Some(TreeEntry {
      inner: TreeEntryInner::Ref(reference),
    })
  }

  #[napi]
  /// Lookup a tree entry by its filename
  pub fn get_path(&self, this_ref: Reference<Tree>, env: Env, name: String) -> Option<TreeEntry> {
    let reference = this_ref
      .share_with(env, |tree| {
        tree
          .inner()
          .get_path(Path::new(&name))
          .convert_without_message()
      })
      .ok()?;
    Some(TreeEntry {
      inner: TreeEntryInner::Ref(reference),
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
    self.inner.next().map(|e| TreeEntry {
      inner: TreeEntryInner::Owned(e),
    })
  }
}

pub(crate) enum TreeEntryInner {
  Owned(git2::TreeEntry<'static>),
  Ref(SharedReference<Tree, git2::TreeEntry<'static>>),
}

#[napi]
pub struct TreeEntry {
  pub(crate) inner: TreeEntryInner,
}

impl Deref for TreeEntryInner {
  type Target = git2::TreeEntry<'static>;

  fn deref(&self) -> &Self::Target {
    match &self {
      TreeEntryInner::Owned(entry) => entry,
      TreeEntryInner::Ref(entry) => entry.deref(),
    }
  }
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
  pub fn name_bytes(&self) -> Uint8Array {
    self.inner.name_bytes().to_vec().into()
  }

  #[napi]
  /// Convert a tree entry to the object it points to.
  pub fn to_object(&self, env: Env, repo: Reference<Repository>) -> Result<GitObject> {
    let object = repo.share_with(env, |repo| {
      self.inner.to_object(&repo.inner).convert_without_message()
    })?;
    Ok(GitObject {
      inner: ObjectParent::Repository(object),
    })
  }
}
