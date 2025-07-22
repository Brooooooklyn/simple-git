use std::ops::Deref;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use chrono::{DateTime, Utc};

use crate::{
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
}

#[napi]
impl Commit {
  #[napi]
  /// Get the id (SHA1) of a repository object
  pub fn id(&self) -> String {
    self.inner.id().to_string()
  }

  #[napi]
  /// Get the id of the tree pointed to by this commit.
  ///
  /// No attempts are made to fetch an object from the ODB.
  pub fn tree_id(&self) -> String {
    self.inner.tree_id().to_string()
  }

  #[napi]
  /// Get the tree pointed to by this commit.
  pub fn tree(&self, this_ref: Reference<Commit>, env: Env) -> Result<Tree> {
    let tree = this_ref.share_with(env, |commit| {
      let tree = commit.inner.tree().convert("Find tree on commit failed")?;
      Ok(tree)
    })?;
    Ok(Tree {
      inner: TreeParent::Commit(tree),
    })
  }

  #[napi]
  // Get the full message of a commit.
  ///
  /// The returned message will be slightly prettified by removing any
  /// potential leading newlines.
  ///
  /// `None` will be returned if the message is not valid utf-8
  pub fn message(&self) -> Option<&str> {
    self.inner.message()
  }

  #[napi]
  /// Get the full message of a commit as a byte slice.
  ///
  /// The returned message will be slightly prettified by removing any
  /// potential leading newlines.
  pub fn message_bytes(&self) -> Buffer {
    self.inner.message_bytes().to_vec().into()
  }

  #[napi]
  /// Get the encoding for the message of a commit, as a string representing a
  /// standard encoding name.
  ///
  /// `None` will be returned if the encoding is not known
  pub fn message_encoding(&self) -> Option<&str> {
    self.inner.message_encoding()
  }

  #[napi]
  /// Get the full raw message of a commit.
  ///
  /// `None` will be returned if the message is not valid utf-8
  pub fn message_raw(&self) -> Option<&str> {
    self.inner.message_raw()
  }

  #[napi]
  /// Get the full raw message of a commit.
  pub fn message_raw_bytes(&self) -> Buffer {
    self.inner.message_raw_bytes().to_vec().into()
  }

  #[napi]
  /// Get the full raw text of the commit header.
  ///
  /// `None` will be returned if the message is not valid utf-8
  pub fn raw_header(&self) -> Option<&str> {
    self.inner.raw_header()
  }

  #[napi]
  /// Get an arbitrary header field.
  pub fn header_field_bytes(&self, field: String) -> Result<Buffer> {
    self
      .inner
      .header_field_bytes(field)
      .map(|b| b.to_vec().into())
      .convert_without_message()
  }

  #[napi]
  /// Get the full raw text of the commit header.
  pub fn raw_header_bytes(&self) -> Buffer {
    self.inner.raw_header_bytes().to_vec().into()
  }

  #[napi]
  /// Get the short "summary" of the git commit message.
  ///
  /// The returned message is the summary of the commit, comprising the first
  /// paragraph of the message with whitespace trimmed and squashed.
  ///
  /// `None` may be returned if an error occurs or if the summary is not valid
  /// utf-8.
  pub fn summary(&self) -> Option<&str> {
    self.inner.summary()
  }

  #[napi]
  /// Get the short "summary" of the git commit message.
  ///
  /// The returned message is the summary of the commit, comprising the first
  /// paragraph of the message with whitespace trimmed and squashed.
  ///
  /// `None` may be returned if an error occurs
  pub fn summary_bytes(&self) -> Option<Buffer> {
    self.inner.summary_bytes().map(|s| s.to_vec().into())
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
  pub fn body(&self) -> Option<&str> {
    self.inner.body()
  }

  #[napi]
  /// Get the long "body" of the git commit message.
  ///
  /// The returned message is the body of the commit, comprising everything
  /// but the first paragraph of the message. Leading and trailing whitespaces
  /// are trimmed.
  ///
  /// `None` may be returned if an error occurs.
  pub fn body_bytes(&self) -> Option<Buffer> {
    self.inner.body_bytes().map(|b| b.to_vec().into())
  }

  #[napi]
  /// Get the commit time (i.e. committer time) of a commit.
  ///
  /// The first element of the tuple is the time, in seconds, since the epoch.
  /// The second element is the offset, in minutes, of the time zone of the
  /// committer's preferred time zone.
  pub fn time(&self) -> Result<DateTime<Utc>> {
    let committer_time = self.inner.time();

    DateTime::from_timestamp(committer_time.seconds(), 0)
      .ok_or_else(|| Error::from_reason("Invalid commit time"))
  }

  #[napi]
  /// Get the author of this commit.
  pub fn author(&self, this_ref: Reference<Commit>, env: Env) -> Result<Signature> {
    let author = this_ref.share_with(env, |commit| Ok(commit.inner.author()))?;
    Ok(Signature {
      inner: SignatureInner::FromCommit(author),
    })
  }

  #[napi]
  /// Get the committer of this commit.
  pub fn committer(&self, this_ref: Reference<Commit>, env: Env) -> Result<Signature> {
    let committer = this_ref.share_with(env, |commit| Ok(commit.inner.committer()))?;
    Ok(Signature {
      inner: SignatureInner::FromCommit(committer),
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
  pub fn parent_count(&self) -> usize {
    self.inner.parent_count()
  }

  #[napi]
  /// Get the specified parent of the commit.
  ///
  /// Use the `parents` iterator to return an iterator over all parents.
  pub fn parent(&self, i: u32) -> Result<Commit> {
    Ok(Self {
      inner: CommitInner::Commit(
        self
          .inner
          .parent(i as usize)
          .convert("Find parent commit failed")?,
      ),
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
  pub fn as_object(&self) -> crate::object::GitObject {
    crate::object::GitObject {
      inner: ObjectParent::Object(self.inner.as_object().clone()),
    }
  }
}
