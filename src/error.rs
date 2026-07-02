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
  use napi_derive::napi;

  /// Stable string tokens surfaced to JS as `error.code`. The first 28 variants
  /// mirror `git2::ErrorCode` (verbatim names); `InvalidArg` is the napi-level
  /// token. `GenericError` doubles as the catch-all. `GitCode: Copy` ⇒ it is
  /// `Send + Sync`, which is required so it can ride along as an async carrier
  /// field on `napi::Error<GitCode>`.
  #[napi(string_enum, js_name = "GitErrorCode")]
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

  /// Runtime type guard for the coded errors this addon throws.
  ///
  /// Returns `true` iff `e` is a genuine `Error` instance — tested with
  /// `instanceof` against the current realm's global `Error` constructor — that
  /// also carries a string `code` property. Returns `false` for non-errors,
  /// plain objects, `null`/`undefined`, and `Error`s without a string `code`.
  ///
  /// This is a structural (shape) guard: it also matches a non-git `Error` that
  /// happens to expose a string `.code` (e.g. Node's `ENOENT`). That is the
  /// accepted, standard trade-off — the valid token set is intentionally not
  /// enumerated. The companion `GitErrorCode` enum lists the tokens this addon
  /// actually produces.
  // napi emits the calling wrapper behind `#[cfg(all(not(test), ...))]`, so under
  // `cfg(test)` nothing references this fn. Unlike the other free `#[napi]` fns
  // (which sit in `pub mod`s and are thus part of the crate's public API), this
  // one lives in the private `mod error`, so `dead_code` fires for the test
  // build only. Silence it there; the non-test build wires it up normally.
  #[cfg_attr(test, allow(dead_code))]
  #[napi(
    ts_args_type = "e: unknown",
    ts_return_type = "e is Error & { code: GitErrorCode }"
  )]
  pub fn is_git_error(env: &Env, e: Unknown) -> napi::Result<bool> {
    // Only objects can be `Error` instances; short-circuit primitives, `null`,
    // and `undefined` before touching `instanceof`/`coerce_to_object`.
    if e.get_type()? != ValueType::Object {
      return Ok(false);
    }
    // Genuine `e instanceof Error` against this realm's global constructor.
    let error_ctor = env
      .get_global()?
      .get_named_property_unchecked::<Unknown>("Error")?;
    if !e.instanceof(error_ctor)? {
      return Ok(false);
    }
    // `typeof e.code === "string"`.
    let code = e
      .coerce_to_object()?
      .get_named_property_unchecked::<Unknown>("code")?;
    Ok(code.get_type()? == ValueType::String)
  }

  use std::sync::atomic::{AtomicBool, Ordering};

  /// The exact error surfaced when a disposed `Repository` — or any handle
  /// derived from it — is accessed after `dispose()`/`free()`. Kept byte-for-byte
  /// identical to the message `Repository::inner()` throws (see repo.rs) so a
  /// disposed repository and every derived handle surface an IDENTICAL error.
  pub(crate) fn disposed_error() -> napi::Error<GitCode> {
    napi::Error::new(
      GitCode::GenericError,
      "Repository has been disposed".to_string(),
    )
  }

  /// Guard used by every derived-handle method that would otherwise deref a
  /// `git2` object borrowed from a now-freed repository. Returns `Ok(())` while
  /// the owning repository is live, else `disposed_error()`.
  ///
  /// `Ordering::Relaxed` is sufficient: the repository and every handle derived
  /// from it live on the single JS main thread, so this flag synchronizes
  /// nothing but its own value — it publishes no cross-thread data.
  pub(crate) fn ensure_alive(alive: &AtomicBool) -> Result<()> {
    if alive.load(Ordering::Relaxed) {
      Ok(())
    } else {
      Err(disposed_error())
    }
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
