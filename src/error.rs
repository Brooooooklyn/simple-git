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
        crate::GitErrorCode::from_git2(err.code()),
        format!("{}: {}", msg.as_ref(), err),
      )
    })
  }

  #[inline]
  fn convert_without_message(self) -> crate::Result<Self::Associate> {
    self.map_err(|err| {
      napi::Error::new(
        crate::GitErrorCode::from_git2(err.code()),
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
/// `crate::GitErrorCode` / `crate::coded_error` / `crate::CodeInto` resolve crate-wide.
pub(crate) mod codes {
  use napi::{Env, Status, bindgen_prelude::*};
  use napi_derive::napi;

  /// Stable string tokens surfaced to JS as `error.code`. The first 28 variants
  /// mirror `git2::ErrorCode` (verbatim names); `InvalidArg` is the napi-level
  /// token. `GenericError` doubles as the catch-all. `GitErrorCode: Copy` ⇒ it is
  /// `Send + Sync`, which is required so it can ride along as an async carrier
  /// field on `napi::Error<GitErrorCode>`.
  #[napi(string_enum)]
  #[derive(Copy, Clone, Debug, PartialEq, Eq)]
  pub enum GitErrorCode {
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

  impl AsRef<str> for GitErrorCode {
    fn as_ref(&self) -> &str {
      match self {
        GitErrorCode::GenericError => "GenericError",
        GitErrorCode::NotFound => "NotFound",
        GitErrorCode::Exists => "Exists",
        GitErrorCode::Ambiguous => "Ambiguous",
        GitErrorCode::BufSize => "BufSize",
        GitErrorCode::User => "User",
        GitErrorCode::BareRepo => "BareRepo",
        GitErrorCode::UnbornBranch => "UnbornBranch",
        GitErrorCode::Unmerged => "Unmerged",
        GitErrorCode::NotFastForward => "NotFastForward",
        GitErrorCode::InvalidSpec => "InvalidSpec",
        GitErrorCode::Conflict => "Conflict",
        GitErrorCode::Locked => "Locked",
        GitErrorCode::Modified => "Modified",
        GitErrorCode::Auth => "Auth",
        GitErrorCode::Certificate => "Certificate",
        GitErrorCode::Applied => "Applied",
        GitErrorCode::Peel => "Peel",
        GitErrorCode::Eof => "Eof",
        GitErrorCode::Invalid => "Invalid",
        GitErrorCode::Uncommitted => "Uncommitted",
        GitErrorCode::Directory => "Directory",
        GitErrorCode::MergeConflict => "MergeConflict",
        GitErrorCode::HashsumMismatch => "HashsumMismatch",
        GitErrorCode::IndexDirty => "IndexDirty",
        GitErrorCode::ApplyFail => "ApplyFail",
        GitErrorCode::Owner => "Owner",
        GitErrorCode::Timeout => "Timeout",
        GitErrorCode::InvalidArg => "InvalidArg",
      }
    }
  }

  impl GitErrorCode {
    /// Map a `git2::ErrorCode` to its token. `git2::ErrorCode` is an exhaustive
    /// (not `#[non_exhaustive]`) enum with 28 variants; matching all 28 *plus* a
    /// wildcard would make the wildcard an `unreachable_pattern` (a warning that
    /// breaks the clippy-clean bar). So `GenericError` is folded into the same
    /// `_ => GitErrorCode::GenericError` catch-all that keeps this forward-compatible
    /// with any variant a future git2 may add. The remaining 27 map 1:1.
    pub fn from_git2(code: git2::ErrorCode) -> Self {
      match code {
        git2::ErrorCode::NotFound => GitErrorCode::NotFound,
        git2::ErrorCode::Exists => GitErrorCode::Exists,
        git2::ErrorCode::Ambiguous => GitErrorCode::Ambiguous,
        git2::ErrorCode::BufSize => GitErrorCode::BufSize,
        git2::ErrorCode::User => GitErrorCode::User,
        git2::ErrorCode::BareRepo => GitErrorCode::BareRepo,
        git2::ErrorCode::UnbornBranch => GitErrorCode::UnbornBranch,
        git2::ErrorCode::Unmerged => GitErrorCode::Unmerged,
        git2::ErrorCode::NotFastForward => GitErrorCode::NotFastForward,
        git2::ErrorCode::InvalidSpec => GitErrorCode::InvalidSpec,
        git2::ErrorCode::Conflict => GitErrorCode::Conflict,
        git2::ErrorCode::Locked => GitErrorCode::Locked,
        git2::ErrorCode::Modified => GitErrorCode::Modified,
        git2::ErrorCode::Auth => GitErrorCode::Auth,
        git2::ErrorCode::Certificate => GitErrorCode::Certificate,
        git2::ErrorCode::Applied => GitErrorCode::Applied,
        git2::ErrorCode::Peel => GitErrorCode::Peel,
        git2::ErrorCode::Eof => GitErrorCode::Eof,
        git2::ErrorCode::Invalid => GitErrorCode::Invalid,
        git2::ErrorCode::Uncommitted => GitErrorCode::Uncommitted,
        git2::ErrorCode::Directory => GitErrorCode::Directory,
        git2::ErrorCode::MergeConflict => GitErrorCode::MergeConflict,
        git2::ErrorCode::HashsumMismatch => GitErrorCode::HashsumMismatch,
        git2::ErrorCode::IndexDirty => GitErrorCode::IndexDirty,
        git2::ErrorCode::ApplyFail => GitErrorCode::ApplyFail,
        git2::ErrorCode::Owner => GitErrorCode::Owner,
        git2::ErrorCode::Timeout => GitErrorCode::Timeout,
        // `GenericError` and any future git2 variant collapse to the catch-all.
        _ => GitErrorCode::GenericError,
      }
    }
  }

  /// Crate-local result whose error carries a `GitErrorCode` (distinct from
  /// `napi::Result<T, S = Status>`, whose error carries a `Status`). Task 2
  /// threads this through the fallible git paths.
  pub type Result<T> = core::result::Result<T, napi::Error<GitErrorCode>>;

  /// Build a `napi::Error<Status>` whose pre-materialised JS error object carries
  /// a `.code` string property. When thrown, napi reuses this object verbatim, so
  /// `.code` survives onto the JS `Error`. The infallible `unwrap_or_else`
  /// fallback guarantees this never panics even if the napi object plumbing fails
  /// (in which case the error is still surfaced, just without `.code`).
  pub fn coded_error(env: Env, code: GitErrorCode, message: String) -> napi::Error {
    (|| -> napi::Result<napi::Error> {
      let mut obj = env.create_error(napi::Error::new(Status::GenericFailure, message.clone()))?;
      obj.set_named_property("code", code.as_ref())?;
      Ok(napi::Error::from(obj.into_unknown(&env)?))
    })()
    .unwrap_or_else(|_| napi::Error::new(Status::GenericFailure, message))
  }

  /// Runtime type guard for the coded errors this addon throws.
  ///
  /// Returns `true` iff `e` is a genuine `Error` object — tested with the
  /// Node-API native-error check (`napi_is_error`, a pure V8 `IsNativeError`
  /// test) — whose `code` is a real member of the `GitErrorCode` enum. The
  /// native check recognizes cross-realm and subclassed errors while rejecting
  /// look-alike proxies and plain objects. Membership is validated against that
  /// generated enum (the single source of truth), so a non-git `Error` (e.g.
  /// Node's `ENOENT`) or an `AbortSignal` cancellation (`code: 'Cancelled'`,
  /// which is a napi-level token, NOT a `GitErrorCode`) returns `false`.
  /// Non-errors, plain objects, `null`/`undefined`, and `Error`s without a
  /// member `code` all return `false`.
  ///
  /// The guard is TOTAL: it never throws for any input. Because the `Error`
  /// check is the native `napi_is_error` (which runs NO JS callbacks), a hostile
  /// value cannot hijack it — a throwing `[[GetPrototypeOf]]`/proxy trap or a
  /// throwing `Error[Symbol.hasInstance]` yields `false`, not a thrown error. An
  /// `Error` whose `code` is a throwing getter likewise yields `false` (the
  /// pending exception is cleared), so it is always safe to call inside a
  /// `catch`.
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
    // Native `Error`-object check via Node-API `napi_is_error` (a pure V8
    // `IsNativeError` test). Unlike a JS-level `instanceof`, it invokes NO user
    // code — no `[[GetPrototypeOf]]`, no proxy traps, no `Symbol.hasInstance` —
    // so it CANNOT throw, and it still recognizes cross-realm/subclassed errors
    // while rejecting look-alike proxies and plain objects. `napi_is_error` sets
    // no pending exception, and a non-ok status carries none either, so collapse
    // any `Err` to `false` (`unwrap_or`). This subsumes the old
    // object/`instanceof` short-circuits: primitives, `null`, `undefined`, plain
    // objects, and proxies all return `false`, totally.
    if !e.is_error().unwrap_or(false) {
      return Ok(false);
    }
    // SOUND + TOTAL membership read. `napi_is_error` above guarantees `e` is an
    // object, so `coerce_to_object` is an identity cast that runs no user code.
    // Reading `.code` as a `GitErrorCode` succeeds ONLY when it is a real member
    // of the generated enum (the single source of truth): the `string_enum`
    // `FromNapiValue` returns `Ok` for a member and `Err(InvalidArg)` — with NO
    // pending exception — for a non-member string, a non-string, or a missing
    // property. A THROWING `code` getter instead surfaces as `Err` WITH a
    // pending JS exception, so clear the pending exception unconditionally
    // (`napi_get_and_clear_last_exception` is a no-op when nothing is pending).
    // This keeps the guard total: it returns a bool and never throws, for every
    // possible input.
    match e
      .coerce_to_object()?
      .get_named_property::<GitErrorCode>("code")
    {
      Ok(_) => Ok(true),
      Err(_) => {
        unsafe {
          let mut pending = std::ptr::null_mut();
          napi::sys::napi_get_and_clear_last_exception(env.raw(), &mut pending);
        }
        Ok(false)
      }
    }
  }

  use std::sync::atomic::{AtomicBool, Ordering};

  /// The exact error surfaced when a disposed `Repository` — or any handle
  /// derived from it — is accessed after `dispose()`/`free()`. Kept byte-for-byte
  /// identical to the message `Repository::inner()` throws (see repo.rs) so a
  /// disposed repository and every derived handle surface an IDENTICAL error.
  pub(crate) fn disposed_error() -> napi::Error<GitErrorCode> {
    napi::Error::new(
      GitErrorCode::GenericError,
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

  /// Collapse a `Result<T>` (error carries a `GitErrorCode`) into a `napi::Result<T>`
  /// whose error still surfaces `.code`. This lets `share_with` closures and the
  /// async outer-converts turn an `Error<GitErrorCode>` into a coded `Error<Status>`.
  pub(crate) trait CodeInto<T> {
    fn code_into(self, env: Env) -> napi::Result<T>;
  }

  impl<T> CodeInto<T> for Result<T> {
    fn code_into(self, env: Env) -> napi::Result<T> {
      // `napi::Error<GitErrorCode>` implements `Drop`, so `reason` can't be moved out
      // by field access (E0509). `status` is `Copy`; take `reason` via `mem::take`.
      self.map_err(|mut e| coded_error(env, e.status, core::mem::take(&mut e.reason)))
    }
  }

  #[cfg(test)]
  mod tests {
    use super::GitErrorCode;

    #[test]
    fn from_git2_maps_representative_codes() {
      assert_eq!(
        GitErrorCode::from_git2(git2::ErrorCode::NotFound),
        GitErrorCode::NotFound
      );
      assert_eq!(
        GitErrorCode::from_git2(git2::ErrorCode::Exists),
        GitErrorCode::Exists
      );
      assert_eq!(
        GitErrorCode::from_git2(git2::ErrorCode::InvalidSpec),
        GitErrorCode::InvalidSpec
      );
      assert_eq!(
        GitErrorCode::from_git2(git2::ErrorCode::Auth),
        GitErrorCode::Auth
      );
      assert_eq!(
        GitErrorCode::from_git2(git2::ErrorCode::Owner),
        GitErrorCode::Owner
      );
      assert_eq!(
        GitErrorCode::from_git2(git2::ErrorCode::Timeout),
        GitErrorCode::Timeout
      );
    }

    #[test]
    fn as_ref_yields_verbatim_tokens() {
      assert_eq!(GitErrorCode::NotFound.as_ref(), "NotFound");
      assert_eq!(GitErrorCode::Exists.as_ref(), "Exists");
      assert_eq!(GitErrorCode::InvalidSpec.as_ref(), "InvalidSpec");
      assert_eq!(GitErrorCode::Auth.as_ref(), "Auth");
      assert_eq!(GitErrorCode::Owner.as_ref(), "Owner");
      assert_eq!(GitErrorCode::Timeout.as_ref(), "Timeout");
      assert_eq!(GitErrorCode::GenericError.as_ref(), "GenericError");
      assert_eq!(GitErrorCode::InvalidArg.as_ref(), "InvalidArg");
    }

    #[test]
    fn unmapped_code_collapses_to_generic() {
      // `git2::ErrorCode` is exhaustive, so no truly "unknown" variant can be
      // constructed here; the `_` catch-all is exercised by `GenericError`, which
      // is intentionally not given its own explicit arm and therefore routes
      // through the wildcard — the same path a future git2 variant would take.
      assert_eq!(
        GitErrorCode::from_git2(git2::ErrorCode::GenericError),
        GitErrorCode::GenericError
      );
    }
  }
}
