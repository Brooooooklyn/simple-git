use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use chrono::{DateTime, Utc};
use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{GitErrorCode, Result, commit::Commit, ensure_alive, error::IntoNapiError};

pub(crate) enum SignatureInner {
  Signature(git2::Signature<'static>),
  FromCommit(SharedReference<Commit, git2::Signature<'static>>),
}

impl Deref for SignatureInner {
  type Target = git2::Signature<'static>;

  fn deref(&self) -> &git2::Signature<'static> {
    match self {
      SignatureInner::Signature(parent) => parent,
      SignatureInner::FromCommit(parent) => parent,
    }
  }
}

#[napi]
/// A Signature is used to indicate authorship of various actions throughout the
/// library.
///
/// Signatures contain a name, email, and timestamp. All fields can be specified
/// with `new` while the `now` constructor omits the timestamp. The
/// [`Repository::signature`] method can be used to create a default signature
/// with name and email values read from the configuration.
///
/// [`Repository::signature`]: struct.Repository.html#method.signature
pub struct Signature {
  pub(crate) inner: SignatureInner,
  /// Liveness flag. For a `FromCommit` signature this is a clone of the owning
  /// commit/repository flag (the signature borrows the commit's memory, so it
  /// must be guarded). For a standalone owned `Signature` (`now`/`new`/
  /// `from_git2`) it is a fresh, never-flipped flag — such a signature borrows
  /// no repository, so it stays valid after any `dispose()`.
  pub(crate) alive: Arc<AtomicBool>,
}

impl Signature {
  /// Wrap an owned `git2::Signature<'static>` (e.g. from
  /// `Repository::signature`) into the napi `Signature` class. Standalone: not
  /// tied to a repository, so it carries a fresh always-live flag.
  pub(crate) fn from_git2(sig: git2::Signature<'static>) -> Signature {
    Signature {
      inner: SignatureInner::Signature(sig),
      alive: Arc::new(AtomicBool::new(true)),
    }
  }
}

#[napi]
impl Signature {
  #[napi(factory)]
  /// Create a new action signature with a timestamp of 'now'.
  ///
  /// See `new` for more information
  pub fn now(name: String, email: String) -> Result<Self> {
    Ok(Signature {
      inner: SignatureInner::Signature(
        git2::Signature::now(name.as_str(), email.as_str()).convert_without_message()?,
      ),
      alive: Arc::new(AtomicBool::new(true)),
    })
  }

  #[napi(constructor)]
  /// Create a new action signature.
  ///
  /// The `time` is a JS `Date`; it is recorded at whole-second resolution with a
  /// zero time-zone offset (UTC).
  ///
  /// Returns error if either `name` or `email` contain angle brackets.
  pub fn new(name: String, email: String, time: DateTime<Utc>) -> Result<Self> {
    Ok(Signature {
      inner: SignatureInner::Signature(
        git2::Signature::new(&name, &email, &git2::Time::new(time.timestamp(), 0))
          .convert_without_message()?,
      ),
      alive: Arc::new(AtomicBool::new(true)),
    })
  }

  #[napi]
  /// Gets the name on the signature.
  ///
  /// Returns `None` if the name is not valid utf-8
  pub fn name(&self) -> Result<Option<&str>> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.name().ok())
  }

  #[napi]
  /// Gets the email on the signature.
  ///
  /// Returns `None` if the email is not valid utf-8
  pub fn email(&self) -> Result<Option<&str>> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.email().ok())
  }

  #[napi]
  /// Return the time the signature was recorded, as a `Date`.
  pub fn when(&self) -> Result<DateTime<Utc>> {
    ensure_alive(&self.alive)?;
    DateTime::from_timestamp(self.inner.when().seconds(), 0)
      .ok_or_else(|| Error::new(GitErrorCode::GenericError, "Invalid signature time"))
  }
}

impl<'a> AsRef<git2::Signature<'a>> for Signature {
  fn as_ref(&self) -> &git2::Signature<'a> {
    &self.inner
  }
}
