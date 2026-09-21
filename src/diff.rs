use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::deltas::Deltas;
use crate::error::IntoNapiError;
use crate::{CodeInto, Result, ensure_alive};

#[napi(object)]
#[derive(Debug, Default)]
pub struct DiffOptions {
  /// Include unmodified files in the diff. Normally unmodified entries are
  /// skipped entirely; when this is `true` they are pulled into the diff (so
  /// they appear in `Diff.deltas()` with an `Unmodified` status) and are also
  /// shown in the listing output formats (name-only, name-status, raw). They
  /// are still never emitted in the patch format.
  pub show_unmodified: Option<bool>,
}

#[napi]
pub struct Diff {
  pub(crate) inner: SharedReference<crate::repo::Repository, git2::Diff<'static>>,
  /// Liveness flag shared with the owning `Repository` (see `Repository::alive`).
  /// Guards every method that derefs the underlying `git2::Diff`.
  pub(crate) alive: Arc<AtomicBool>,
}

#[napi]
impl Diff {
  #[napi]
  /// Merge one diff into another.
  ///
  /// This merges items from the "from" list into the "self" list.  The
  /// resulting diff will have all items that appear in either list.
  /// If an item appears in both lists, then it will be "merged" to appear
  /// as if the old version was from the "onto" list and the new version
  /// is from the "from" list (with the exception that if the item has a
  /// pending DELETE in the middle, then it will show as deleted).
  pub fn merge(&mut self, diff: &Diff) -> Result<()> {
    ensure_alive(&self.alive)?;
    ensure_alive(&diff.alive)?;
    self
      .inner
      .merge(diff.inner.deref())
      .convert_without_message()
  }

  #[napi]
  /// Returns an iterator over the deltas in this diff.
  pub fn deltas(&self, env: Env, self_ref: Reference<Diff>) -> napi::Result<Deltas> {
    ensure_alive(&self.alive).code_into(env)?;
    Ok(Deltas {
      inner: self_ref.share_with(env, |diff| Ok(diff.inner.deltas()))?,
      alive: self.alive.clone(),
    })
  }

  #[napi]
  /// Check if deltas are sorted case sensitively or insensitively.
  pub fn is_sorted_icase(&self) -> Result<bool> {
    ensure_alive(&self.alive)?;
    Ok(self.inner.is_sorted_icase())
  }

  #[napi]
  /// Render the diff as unified-diff bytes (the `git diff` patch format).
  ///
  /// `git_diff_line::content` does not include the origin sigil, so for
  /// context/add/delete lines the `+`/`-`/space sigil is emitted before the
  /// content bytes, matching libgit2's `diff_print.c`.
  ///
  /// Returned as raw bytes rather than a string so non-UTF-8 file content is
  /// preserved exactly.
  pub fn to_patch(&self) -> Result<Buffer> {
    ensure_alive(&self.alive)?;
    let mut bytes = Vec::new();
    self
      .inner
      .print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = line.origin();
        if matches!(origin, ' ' | '+' | '-') {
          let mut buf = [0u8; 4];
          bytes.extend_from_slice(origin.encode_utf8(&mut buf).as_bytes());
        }
        bytes.extend_from_slice(line.content());
        true
      })
      .convert_without_message()?;
    Ok(bytes.into())
  }
}
