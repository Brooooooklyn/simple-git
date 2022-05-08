use std::ops::Deref;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::deltas::Deltas;
use crate::error::IntoNapiError;

#[napi(object)]
#[derive(Debug, Default)]
pub struct DiffOptions {
  /// When generating output, include the names of unmodified files if they
  /// are included in the `Diff`. Normally these are skipped in the formats
  /// that list files (e.g. name-only, name-status, raw). Even with this these
  /// will not be included in the patch format.
  pub show_unmodified: Option<bool>,
}

#[napi]
pub struct Diff {
  pub(crate) inner: SharedReference<crate::repo::Repository, git2::Diff<'static>>,
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
    self
      .inner
      .merge(diff.inner.deref())
      .convert_without_message()
  }

  #[napi]
  /// Returns an iterator over the deltas in this diff.
  pub fn deltas(&self, env: Env, self_ref: Reference<Diff>) -> Result<Deltas> {
    Ok(Deltas {
      inner: self_ref.share_with(env, |diff| Ok(diff.inner.deltas()))?,
    })
  }

  #[napi]
  /// Check if deltas are sorted case sensitively or insensitively.
  pub fn is_sorted_icase(&self) -> bool {
    self.inner.is_sorted_icase()
  }
}
