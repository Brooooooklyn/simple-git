use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use chrono::{DateTime, Utc};

use crate::{
  CodeInto, GitErrorCode, Result, ensure_alive,
  error::IntoNapiError,
  object::ObjectParent,
  signature::{Signature, SignatureInner},
  tree::{Tree, TreeParent},
};

pub(crate) enum CommitInner {
  Repository(SharedReference<crate::repo::Repository, git2::Commit<'static>>),
  Commit(git2::Commit<'static>),
}

impl Deref for CommitInner {
  type Target = git2::Commit<'static>;

  fn deref(&self) -> &Self::Target {
    match self {
      CommitInner::Repository(r) => r.deref(),
      CommitInner::Commit(c) => c,
    }
  }
}

#[napi]
pub struct Commit {
  pub(crate) inner: CommitInner,
  /// Liveness flag shared with the owning `Repository` (see `Repository::alive`).
  /// Both the `Repository` and owned `Commit` variants point into the repo's
  /// odb, so both are guarded by this flag.
  pub(crate) alive: Arc<AtomicBool>,
}

#[napi]
impl Commit {
  #[napi]
  /// Get the id (SHA1) of a repository object
  pub fn id(&self) -> Result<String> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.id().to_string())
  }

  #[napi]
  /// Get the id of the tree pointed to by this commit.
  ///
  /// No attempts are made to fetch an object from the ODB.
  pub fn tree_id(&self) -> Result<String> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.tree_id().to_string())
  }

  #[napi]
  /// Get the tree pointed to by this commit.
  pub fn tree(&self, this_ref: Reference<Commit>, env: Env) -> napi::Result<Tree> {
    ensure_alive(&self.alive).code_into(env)?;
    let tree = this_ref.share_with(env, |commit| {
      let tree = commit
        .inner
        .tree()
        .convert("Find tree on commit failed")
        .code_into(env)?;
      Ok(tree)
    })?;
    Ok(Tree {
      inner: TreeParent::Commit(tree),
      alive: self.alive.clone(),
    })
  }

  #[napi]
  /// Get the full message of a commit.
  ///
  /// The returned message will be slightly prettified by removing any
  /// potential leading newlines.
  ///
  /// `None` will be returned if the message is not valid utf-8
  pub fn message(&self) -> Result<Option<&str>> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.message().ok())
  }

  #[napi]
  /// Get the full message of a commit as a byte slice.
  ///
  /// The returned message will be slightly prettified by removing any
  /// potential leading newlines.
  pub fn message_bytes(&self) -> Result<Buffer> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.message_bytes().to_vec().into())
  }

  #[napi]
  /// Get the encoding for the message of a commit, as a string representing a
  /// standard encoding name.
  ///
  /// `None` will be returned if the encoding is not known
  pub fn message_encoding(&self) -> Result<Option<&str>> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.message_encoding().ok().flatten())
  }

  #[napi]
  /// Get the full raw message of a commit.
  ///
  /// `None` will be returned if the message is not valid utf-8
  pub fn message_raw(&self) -> Result<Option<&str>> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.message_raw().ok())
  }

  #[napi]
  /// Get the full raw message of a commit.
  pub fn message_raw_bytes(&self) -> Result<Buffer> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.message_raw_bytes().to_vec().into())
  }

  #[napi]
  /// Get the full raw text of the commit header.
  ///
  /// `None` will be returned if the message is not valid utf-8
  pub fn raw_header(&self) -> Result<Option<&str>> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.raw_header().ok())
  }

  #[napi]
  /// Get an arbitrary header field.
  pub fn header_field_bytes(&self, field: String) -> Result<Buffer> {
    ensure_alive(&self.alive)?;
    self
      .inner
      .header_field_bytes(field)
      .map(|b| b.to_vec().into())
      .convert_without_message()
  }

  #[napi]
  /// Get the full raw text of the commit header.
  pub fn raw_header_bytes(&self) -> Result<Buffer> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.raw_header_bytes().to_vec().into())
  }

  #[napi]
  /// Get the short "summary" of the git commit message.
  ///
  /// The returned message is the summary of the commit, comprising the first
  /// paragraph of the message with whitespace trimmed and squashed.
  ///
  /// `None` may be returned if an error occurs or if the summary is not valid
  /// utf-8.
  pub fn summary(&self) -> Result<Option<&str>> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.summary().ok().flatten())
  }

  #[napi]
  /// Get the short "summary" of the git commit message.
  ///
  /// The returned message is the summary of the commit, comprising the first
  /// paragraph of the message with whitespace trimmed and squashed.
  ///
  /// `None` may be returned if an error occurs
  pub fn summary_bytes(&self) -> Result<Option<Buffer>> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.summary_bytes().map(|s| s.to_vec().into()))
  }

  #[napi]
  /// Get the long "body" of the git commit message.
  ///
  /// The returned message is the body of the commit, comprising everything
  /// but the first paragraph of the message. Leading and trailing whitespaces
  /// are trimmed.
  ///
  /// `None` may be returned if an error occurs or if the summary is not valid
  /// utf-8.
  pub fn body(&self) -> Result<Option<&str>> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.body().ok().flatten())
  }

  #[napi]
  /// Get the long "body" of the git commit message.
  ///
  /// The returned message is the body of the commit, comprising everything
  /// but the first paragraph of the message. Leading and trailing whitespaces
  /// are trimmed.
  ///
  /// `None` may be returned if an error occurs.
  pub fn body_bytes(&self) -> Result<Option<Buffer>> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.body_bytes().map(|b| b.to_vec().into()))
  }

  #[napi]
  /// Get the commit time (i.e. committer time) of a commit.
  ///
  /// Returns the committer time as a UTC `Date`; the committer's timezone
  /// offset is not preserved (the value is normalized to UTC).
  pub fn time(&self) -> Result<DateTime<Utc>> {
    ensure_alive(&self.alive)?;
    let committer_time = self.inner.time();

    DateTime::from_timestamp(committer_time.seconds(), 0)
      .ok_or_else(|| Error::new(GitErrorCode::GenericError, "Invalid commit time"))
  }

  #[napi]
  /// Get the author of this commit.
  pub fn author(&self, this_ref: Reference<Commit>, env: Env) -> napi::Result<Signature> {
    ensure_alive(&self.alive).code_into(env)?;
    let author = this_ref.share_with(env, |commit| Ok(commit.inner.author()))?;
    Ok(Signature {
      inner: SignatureInner::FromCommit(author),
      alive: self.alive.clone(),
    })
  }

  #[napi]
  /// Get the committer of this commit.
  pub fn committer(&self, this_ref: Reference<Commit>, env: Env) -> napi::Result<Signature> {
    ensure_alive(&self.alive).code_into(env)?;
    let committer = this_ref.share_with(env, |commit| Ok(commit.inner.committer()))?;
    Ok(Signature {
      inner: SignatureInner::FromCommit(committer),
      alive: self.alive.clone(),
    })
  }

  #[napi]
  /// Amend this existing commit with all non-`None` values
  ///
  /// This creates a new commit that is exactly the same as the old commit,
  /// except that any non-`None` values will be updated. The new commit has
  /// the same parents as the old commit.
  ///
  /// For information about `update_ref`, see [`Repository::commit`].
  ///
  /// [`Repository::commit`]: struct.Repository.html#method.commit
  pub fn amend(
    &self,
    update_ref: Option<String>,
    author: Option<&Signature>,
    committer: Option<&Signature>,
    message_encoding: Option<String>,
    message: Option<String>,
    tree: Option<&Tree>,
  ) -> Result<String> {
    ensure_alive(&self.alive)?;
    // Guard the argument handles: an author/committer/tree from a disposed
    // repository would otherwise deref freed git2 state (arg-side UAF).
    if let Some(a) = author {
      ensure_alive(&a.alive)?;
    }
    if let Some(c) = committer {
      ensure_alive(&c.alive)?;
    }
    if let Some(t) = tree {
      ensure_alive(&t.alive)?;
    }
    self
      .inner
      .amend(
        update_ref.as_deref(),
        author.map(|s| &*s.inner),
        committer.map(|s| &*s.inner),
        message_encoding.as_deref(),
        message.as_deref(),
        tree.map(|s| s.inner()),
      )
      .map(|oid| oid.to_string())
      .convert("Amend commit failed")
  }

  #[napi]
  /// Get the number of parents of this commit.
  ///
  /// Use the `parents` iterator to return an iterator over all parents.
  pub fn parent_count(&self) -> Result<u32> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.parent_count() as u32)
  }

  #[napi]
  /// Get the specified parent of the commit.
  ///
  /// Use the `parents` iterator to return an iterator over all parents.
  pub fn parent(&self, i: u32) -> Result<Commit> {
    ensure_alive(&self.alive)?;
    Ok(Self {
      inner: CommitInner::Commit(
        self
          .inner
          .parent(i as usize)
          .convert("Find parent commit failed")?,
      ),
      alive: self.alive.clone(),
    })
  }

  #[napi]
  /// Get the specified parent id of the commit.
  ///
  /// This is different from `parent`, which will attempt to load the
  /// parent commit from the ODB.
  ///
  /// Use the `parent_ids` iterator to return an iterator over all parents.
  pub fn parent_id(&self, i: u32) -> Result<String> {
    ensure_alive(&self.alive)?;
    Ok(
      self
        .inner
        .parent_id(i as usize)
        .convert("Find parent commit failed")?
        .to_string(),
    )
  }

  #[napi]
  /// Casts this Commit to be usable as an `Object`
  pub fn as_object(&self) -> Result<crate::object::GitObject> {
    ensure_alive(&self.alive)?;
    Ok(crate::object::GitObject {
      inner: ObjectParent::Object(self.inner.as_object().clone()),
      alive: self.alive.clone(),
    })
  }
}
