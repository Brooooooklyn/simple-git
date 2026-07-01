pub(crate) trait IntoNapiError: Sized {
  type Associate;

  fn convert<S: AsRef<str>>(self, msg: S) -> crate::Result<Self::Associate>;

  fn convert_without_message(self) -> crate::Result<Self::Associate>;
}

impl<T> IntoNapiError for Result<T, git2::Error> {
  type Associate = T;

  #[inline]
  fn convert<S: AsRef<str>>(self, msg: S) -> crate::Result<T> {
    self.map_err(|err| {
      napi::Error::new(
        crate::GitCode::from_git2(err.code()),
        format!("{}: {}", msg.as_ref(), err),
      )
    })
  }

  #[inline]
  fn convert_without_message(self) -> crate::Result<Self::Associate> {
    self.map_err(|err| {
      napi::Error::new(
        crate::GitCode::from_git2(err.code()),
        format!("libgit2 error: {err}"),
      )
    })
  }
}

pub trait NotNullError {
  type Associate;

  fn expect_not_null(self, msg: String) -> crate::Result<Self::Associate>;
}

impl<T> NotNullError for Option<T> {
  type Associate = T;

  #[inline]
  fn expect_not_null(self, msg: String) -> crate::Result<T> {
    self.ok_or_else(|| napi::Error::new(crate::GitCode::NotFound, msg))
  }
}

/// Coded-error primitives.
///
/// These live in a nested module on purpose. `error.rs` above uses the 2-arg
/// prelude `Result<T, E>` in the `impl ... for Result<T, git2::Error>` receiver;
/// a module-level `type Result<T>` would shadow that prelude `Result`. Nesting
/// keeps the `Result` alias out of the parent module's scope so both coexist,
/// while `lib.rs` re-exports these at the crate root so `crate::Result` /
/// `crate::GitCode` / `crate::coded_error` / `crate::CodeInto` resolve crate-wide.
pub(crate) mod codes {
  use napi::{Env, Status, bindgen_prelude::*};

  /// Stable string tokens surfaced to JS as `error.code`. The first 28 variants
  /// mirror `git2::ErrorCode` (verbatim names); `InvalidArg` is the napi-level
  /// token. `GenericError` doubles as the catch-all. `GitCode: Copy` ⇒ it is
  /// `Send + Sync`, which is required so it can ride along as an async carrier
  /// field on `napi::Error<GitCode>`.
  #[derive(Copy, Clone, Debug, PartialEq, Eq)]
  pub enum GitCode {
    GenericError,
    NotFound,
    Exists,
    Ambiguous,
    BufSize,
    User,
    BareRepo,
    UnbornBranch,
    Unmerged,
    NotFastForward,
    InvalidSpec,
    Conflict,
    Locked,
    Modified,
    Auth,
    Certificate,
    Applied,
    Peel,
    Eof,
    Invalid,
    Uncommitted,
    Directory,
    MergeConflict,
    HashsumMismatch,
    IndexDirty,
    ApplyFail,
    Owner,
    Timeout,
    InvalidArg,
  }

  impl AsRef<str> for GitCode {
    fn as_ref(&self) -> &str {
      match self {
        GitCode::GenericError => "GenericError",
        GitCode::NotFound => "NotFound",
        GitCode::Exists => "Exists",
        GitCode::Ambiguous => "Ambiguous",
        GitCode::BufSize => "BufSize",
        GitCode::User => "User",
        GitCode::BareRepo => "BareRepo",
        GitCode::UnbornBranch => "UnbornBranch",
        GitCode::Unmerged => "Unmerged",
        GitCode::NotFastForward => "NotFastForward",
        GitCode::InvalidSpec => "InvalidSpec",
        GitCode::Conflict => "Conflict",
        GitCode::Locked => "Locked",
        GitCode::Modified => "Modified",
        GitCode::Auth => "Auth",
        GitCode::Certificate => "Certificate",
        GitCode::Applied => "Applied",
        GitCode::Peel => "Peel",
        GitCode::Eof => "Eof",
        GitCode::Invalid => "Invalid",
        GitCode::Uncommitted => "Uncommitted",
        GitCode::Directory => "Directory",
        GitCode::MergeConflict => "MergeConflict",
        GitCode::HashsumMismatch => "HashsumMismatch",
        GitCode::IndexDirty => "IndexDirty",
        GitCode::ApplyFail => "ApplyFail",
        GitCode::Owner => "Owner",
        GitCode::Timeout => "Timeout",
        GitCode::InvalidArg => "InvalidArg",
      }
    }
  }

  impl GitCode {
    /// Map a `git2::ErrorCode` to its token. `git2::ErrorCode` is an exhaustive
    /// (not `#[non_exhaustive]`) enum with 28 variants; matching all 28 *plus* a
    /// wildcard would make the wildcard an `unreachable_pattern` (a warning that
    /// breaks the clippy-clean bar). So `GenericError` is folded into the same
    /// `_ => GitCode::GenericError` catch-all that keeps this forward-compatible
    /// with any variant a future git2 may add. The remaining 27 map 1:1.
    pub fn from_git2(code: git2::ErrorCode) -> Self {
      match code {
        git2::ErrorCode::NotFound => GitCode::NotFound,
        git2::ErrorCode::Exists => GitCode::Exists,
        git2::ErrorCode::Ambiguous => GitCode::Ambiguous,
        git2::ErrorCode::BufSize => GitCode::BufSize,
        git2::ErrorCode::User => GitCode::User,
        git2::ErrorCode::BareRepo => GitCode::BareRepo,
        git2::ErrorCode::UnbornBranch => GitCode::UnbornBranch,
        git2::ErrorCode::Unmerged => GitCode::Unmerged,
        git2::ErrorCode::NotFastForward => GitCode::NotFastForward,
        git2::ErrorCode::InvalidSpec => GitCode::InvalidSpec,
        git2::ErrorCode::Conflict => GitCode::Conflict,
        git2::ErrorCode::Locked => GitCode::Locked,
        git2::ErrorCode::Modified => GitCode::Modified,
        git2::ErrorCode::Auth => GitCode::Auth,
        git2::ErrorCode::Certificate => GitCode::Certificate,
        git2::ErrorCode::Applied => GitCode::Applied,
        git2::ErrorCode::Peel => GitCode::Peel,
        git2::ErrorCode::Eof => GitCode::Eof,
        git2::ErrorCode::Invalid => GitCode::Invalid,
        git2::ErrorCode::Uncommitted => GitCode::Uncommitted,
        git2::ErrorCode::Directory => GitCode::Directory,
        git2::ErrorCode::MergeConflict => GitCode::MergeConflict,
        git2::ErrorCode::HashsumMismatch => GitCode::HashsumMismatch,
        git2::ErrorCode::IndexDirty => GitCode::IndexDirty,
        git2::ErrorCode::ApplyFail => GitCode::ApplyFail,
        git2::ErrorCode::Owner => GitCode::Owner,
        git2::ErrorCode::Timeout => GitCode::Timeout,
        // `GenericError` and any future git2 variant collapse to the catch-all.
        _ => GitCode::GenericError,
      }
    }
  }

  /// Crate-local result whose error carries a `GitCode` (distinct from
  /// `napi::Result<T, S = Status>`, whose error carries a `Status`). Task 2
  /// threads this through the fallible git paths.
  pub type Result<T> = core::result::Result<T, napi::Error<GitCode>>;

  /// Build a `napi::Error<Status>` whose pre-materialised JS error object carries
  /// a `.code` string property. When thrown, napi reuses this object verbatim, so
  /// `.code` survives onto the JS `Error`. The infallible `unwrap_or_else`
  /// fallback guarantees this never panics even if the napi object plumbing fails
  /// (in which case the error is still surfaced, just without `.code`).
  pub fn coded_error(env: Env, code: GitCode, message: String) -> napi::Error {
    (|| -> napi::Result<napi::Error> {
      let mut obj = env.create_error(napi::Error::new(Status::GenericFailure, message.clone()))?;
      obj.set_named_property("code", code.as_ref())?;
      Ok(napi::Error::from(obj.into_unknown(&env)?))
    })()
    .unwrap_or_else(|_| napi::Error::new(Status::GenericFailure, message))
  }

  /// Collapse a `Result<T>` (error carries a `GitCode`) into a `napi::Result<T>`
  /// whose error still surfaces `.code`. This lets `share_with` closures and the
  /// async outer-converts turn an `Error<GitCode>` into a coded `Error<Status>`.
  pub(crate) trait CodeInto<T> {
    fn code_into(self, env: Env) -> napi::Result<T>;
  }

  impl<T> CodeInto<T> for Result<T> {
    fn code_into(self, env: Env) -> napi::Result<T> {
      // `napi::Error<GitCode>` implements `Drop`, so `reason` can't be moved out
      // by field access (E0509). `status` is `Copy`; take `reason` via `mem::take`.
      self.map_err(|mut e| coded_error(env, e.status, core::mem::take(&mut e.reason)))
    }
  }

  #[cfg(test)]
  mod tests {
    use super::GitCode;

    #[test]
    fn from_git2_maps_representative_codes() {
      assert_eq!(
        GitCode::from_git2(git2::ErrorCode::NotFound),
        GitCode::NotFound
      );
      assert_eq!(GitCode::from_git2(git2::ErrorCode::Exists), GitCode::Exists);
      assert_eq!(
        GitCode::from_git2(git2::ErrorCode::InvalidSpec),
        GitCode::InvalidSpec
      );
      assert_eq!(GitCode::from_git2(git2::ErrorCode::Auth), GitCode::Auth);
      assert_eq!(GitCode::from_git2(git2::ErrorCode::Owner), GitCode::Owner);
      assert_eq!(
        GitCode::from_git2(git2::ErrorCode::Timeout),
        GitCode::Timeout
      );
    }

    #[test]
    fn as_ref_yields_verbatim_tokens() {
      assert_eq!(GitCode::NotFound.as_ref(), "NotFound");
      assert_eq!(GitCode::Exists.as_ref(), "Exists");
      assert_eq!(GitCode::InvalidSpec.as_ref(), "InvalidSpec");
      assert_eq!(GitCode::Auth.as_ref(), "Auth");
      assert_eq!(GitCode::Owner.as_ref(), "Owner");
      assert_eq!(GitCode::Timeout.as_ref(), "Timeout");
      assert_eq!(GitCode::GenericError.as_ref(), "GenericError");
      assert_eq!(GitCode::InvalidArg.as_ref(), "InvalidArg");
    }

    #[test]
    fn unmapped_code_collapses_to_generic() {
      // `git2::ErrorCode` is exhaustive, so no truly "unknown" variant can be
      // constructed here; the `_` catch-all is exercised by `GenericError`, which
      // is intentionally not given its own explicit arm and therefore routes
      // through the wildcard — the same path a future git2 variant would take.
      assert_eq!(
        GitCode::from_git2(git2::ErrorCode::GenericError),
        GitCode::GenericError
      );
    }
  }
}
