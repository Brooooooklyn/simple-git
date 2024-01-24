use std::ops::Deref;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{commit::Commit, error::IntoNapiError};

pub(crate) enum SignatureInner {
  Signature(git2::Signature<'static>),
  FromCommit(SharedReference<Commit, git2::Signature<'static>>),
}

impl Deref for SignatureInner {
  type Target = git2::Signature<'static>;

  fn deref(&self) -> &git2::Signature<'static> {
    match self {
      SignatureInner::Signature(parent) => parent,
      SignatureInner::FromCommit(parent) => &*parent,
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
    })
  }

  #[napi(constructor)]
  /// Create a new action signature.
  ///
  /// The `time` specified is in seconds since the epoch, and the `offset` is
  /// the time zone offset in minutes.
  ///
  /// Returns error if either `name` or `email` contain angle brackets.
  pub fn new(name: String, email: String, time: i64) -> Result<Self> {
    Ok(Signature {
      inner: SignatureInner::Signature(
        git2::Signature::new(&name, &email, &git2::Time::new(time, 0)).convert_without_message()?,
      ),
    })
  }

  #[napi]
  /// Gets the name on the signature.
  ///
  /// Returns `None` if the name is not valid utf-8
  pub fn name(&self) -> Option<&str> {
    self.inner.name()
  }

  #[napi]
  /// Gets the email on the signature.
  ///
  /// Returns `None` if the email is not valid utf-8
  pub fn email(&self) -> Option<&str> {
    self.inner.email()
  }

  #[napi]
  /// Return the time, in seconds, from epoch
  pub fn when(&self) -> i64 {
    self.inner.when().seconds()
  }
}

impl<'a> AsRef<git2::Signature<'a>> for Signature {
  fn as_ref(&self) -> &git2::Signature<'a> {
    &self.inner
  }
}
