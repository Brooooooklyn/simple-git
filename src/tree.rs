use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use napi::bindgen_prelude::{Buffer, Env, Error, Generator, Reference, Result, SharedReference};
use napi_derive::napi;

use crate::{
  CodeInto, ensure_alive,
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
  /// Liveness flag shared with the owning `Repository` (see `Repository::alive`).
  /// Guards every method that derefs the underlying `git2::Tree`, regardless of
  /// which `TreeParent` (Repository/Reference/Commit) rooted it — they all share
  /// the same repository's flag.
  pub(crate) alive: Arc<AtomicBool>,
}

#[napi]
impl Tree {
  pub(crate) fn inner<'repo>(&'repo self) -> &'repo git2::Tree<'repo> {
    match &self.inner {
      TreeParent::Repository(parent) => parent,
      TreeParent::Reference(parent) => parent,
      TreeParent::Commit(parent) => parent,
    }
  }

  #[napi]
  /// Get the id (SHA1) of a repository object
  pub fn id(&self) -> crate::Result<String> {
    ensure_alive(&self.alive)?;
    Ok(self.inner().id().to_string())
  }

  #[napi]
  /// Get the number of entries listed in a tree.
  pub fn size(&self) -> crate::Result<u32> {
    ensure_alive(&self.alive)?;
    Ok(self.inner().len() as u32)
  }

  #[napi]
  /// Return `true` if there is not entry
  pub fn is_empty(&self) -> crate::Result<bool> {
    ensure_alive(&self.alive)?;
    Ok(self.inner().is_empty())
  }

  #[napi]
  /// Returns an iterator over the entries in this tree.
  pub fn entries(&self, this_ref: Reference<Tree>, env: Env) -> Result<TreeIter> {
    ensure_alive(&self.alive).code_into(env)?;
    Ok(TreeIter {
      inner: this_ref.share_with(env, |tree| Ok(tree.inner().iter()))?,
      alive: self.alive.clone(),
    })
  }

  #[napi]
  /// Lookup a tree entry by SHA value
  pub fn get_id(
    &self,
    this_ref: Reference<Tree>,
    env: Env,
    id: String,
  ) -> crate::Result<Option<TreeEntry>> {
    ensure_alive(&self.alive)?;
    let alive = self.alive.clone();
    let reference = this_ref
      .share_with(env, |tree| {
        if let Some(entry) = tree.inner().get_id(
          git2::Oid::from_str(&id)
            .convert_without_message()
            .code_into(env)?,
        ) {
          Ok(entry)
        } else {
          Err(Error::new(napi::Status::InvalidArg, "Tree entry not found"))
        }
      })
      .ok();
    Ok(reference.map(|reference| TreeEntry {
      inner: TreeEntryInner::Ref(reference),
      alive,
    }))
  }

  #[napi]
  /// Lookup a tree entry by its position in the tree
  pub fn get(
    &self,
    this_ref: Reference<Tree>,
    env: Env,
    index: u32,
  ) -> crate::Result<Option<TreeEntry>> {
    ensure_alive(&self.alive)?;
    let alive = self.alive.clone();
    let reference = this_ref
      .share_with(env, |tree| {
        if let Some(entry) = tree.inner().get(index as usize) {
          Ok(entry)
        } else {
          Err(Error::new(napi::Status::InvalidArg, "Tree entry not found"))
        }
      })
      .ok();
    Ok(reference.map(|reference| TreeEntry {
      inner: TreeEntryInner::Ref(reference),
      alive,
    }))
  }

  #[napi]
  /// Lookup a direct child entry of this tree by its name.
  ///
  /// `name` is a single path component (a filename), not a multi-component
  /// path; this does not descend into subtrees. To follow a relative path
  /// through nested subtrees, use `getPath`.
  pub fn get_name(
    &self,
    this_ref: Reference<Tree>,
    env: Env,
    name: String,
  ) -> crate::Result<Option<TreeEntry>> {
    ensure_alive(&self.alive)?;
    let alive = self.alive.clone();
    let reference = this_ref
      .share_with(env, |tree| {
        if let Some(entry) = tree.inner().get_name(&name) {
          Ok(entry)
        } else {
          Err(Error::new(napi::Status::InvalidArg, "Tree entry not found"))
        }
      })
      .ok();
    Ok(reference.map(|reference| TreeEntry {
      inner: TreeEntryInner::Ref(reference),
      alive,
    }))
  }

  #[napi]
  /// Lookup a tree entry by a relative path, descending through subtrees.
  ///
  /// `name` is a path relative to this tree and may contain multiple
  /// components (e.g. `src/lib.rs`); each component is resolved in turn,
  /// walking into nested subtrees. To look up a direct child by its name,
  /// use `getName`.
  pub fn get_path(
    &self,
    this_ref: Reference<Tree>,
    env: Env,
    name: String,
  ) -> crate::Result<Option<TreeEntry>> {
    ensure_alive(&self.alive)?;
    let alive = self.alive.clone();
    let reference = this_ref
      .share_with(env, |tree| {
        tree
          .inner()
          .get_path(Path::new(&name))
          .convert_without_message()
          .code_into(env)
      })
      .ok();
    Ok(reference.map(|reference| TreeEntry {
      inner: TreeEntryInner::Ref(reference),
      alive,
    }))
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
  /// Liveness flag shared with the owning `Repository` (see `Repository::alive`).
  pub(crate) alive: Arc<AtomicBool>,
}

#[napi]
impl Generator for TreeIter {
  type Yield = TreeEntry;
  type Return = ();
  type Next = ();

  fn next(&mut self, _value: Option<()>) -> Option<Self::Yield> {
    // `Generator::next` returns `Option`, not `Result`, so it cannot throw. On
    // disposal the iterator borrows a freed repo, so returning `None` (a safe
    // iteration end) is the correct memory-safe substitute for a throw — it
    // prevents the use-after-free deref below.
    if !self.alive.load(Ordering::Relaxed) {
      return None;
    }
    let alive = self.alive.clone();
    self.inner.next().map(|e| TreeEntry {
      inner: TreeEntryInner::Owned(e),
      alive,
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
  /// Liveness flag shared with the owning `Repository` (see `Repository::alive`).
  /// Both the `Owned` and `Ref` variants point into the repo's odb, so both are
  /// guarded by this flag.
  pub(crate) alive: Arc<AtomicBool>,
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
  pub fn id(&self) -> crate::Result<String> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.id().to_string())
  }

  #[napi]
  /// Get the name of a tree entry
  pub fn name(&self) -> crate::Result<&str> {
    ensure_alive(&self.alive)?;
    self
      .inner
      .name()
      .ok()
      .ok_or_else(|| Error::new(crate::GitErrorCode::GenericError, "Invalid utf-8"))
  }

  #[napi]
  /// Get the filename of a tree entry
  pub fn name_bytes(&self) -> crate::Result<Buffer> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.name_bytes().to_vec().into())
  }

  #[napi]
  /// Convert a tree entry to the object it points to.
  pub fn to_object(&self, env: Env, repo: Reference<Repository>) -> Result<GitObject> {
    ensure_alive(&self.alive).code_into(env)?;
    // §4b-special: the produced object lives in the PASSED repo's odb, so its
    // liveness is that repo's flag — capture it before `share_with` consumes the
    // `Reference<Repository>`.
    let alive = repo.alive.clone();
    let object = repo.share_with(env, |repo| {
      self
        .inner
        .to_object(repo.inner().code_into(env)?)
        .convert_without_message()
        .code_into(env)
    })?;
    Ok(GitObject {
      inner: ObjectParent::Repository(object),
      alive,
    })
  }
}
