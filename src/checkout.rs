use std::path::Path;

use napi_derive::napi;

/// Options controlling how a checkout writes files into the working directory.
///
/// The default is a **safe** checkout (matching `git checkout`): files with
/// local modifications are left untouched. Set `force` to overwrite them, which
/// can discard uncommitted changes — use it deliberately.
#[napi(object)]
pub struct CheckoutOptions {
  /// Force the checkout, overwriting any local changes in the working tree.
  /// Defaults to a safe checkout when omitted or `false`.
  pub force: Option<bool>,
  /// Recreate files that are missing from the working tree even in a safe
  /// checkout.
  pub recreate_missing: Option<bool>,
  /// Allow the checkout to write files that conflict with the working tree.
  pub allow_conflicts: Option<bool>,
  /// Restrict the checkout to these pathspecs. When omitted, all paths are
  /// checked out.
  pub paths: Option<Vec<String>>,
  /// Write the checked-out files into this directory instead of the
  /// repository's working directory.
  pub target_dir: Option<String>,
}

/// Translate `CheckoutOptions` into a libgit2 `CheckoutBuilder`.
///
/// IMPORTANT: the builder starts in libgit2's **safe** mode. We only call
/// `.force()` when `force == Some(true)`; forcing by default would silently
/// overwrite uncommitted working-tree changes.
pub(crate) fn build_checkout_builder(
  options: Option<CheckoutOptions>,
) -> git2::build::CheckoutBuilder<'static> {
  let mut builder = git2::build::CheckoutBuilder::new();
  let Some(options) = options else {
    return builder;
  };
  if options.force.unwrap_or(false) {
    builder.force();
  }
  if options.recreate_missing.unwrap_or(false) {
    builder.recreate_missing(true);
  }
  if options.allow_conflicts.unwrap_or(false) {
    builder.allow_conflicts(true);
  }
  if let Some(paths) = options.paths {
    for path in paths {
      builder.path(path);
    }
  }
  if let Some(target_dir) = options.target_dir {
    builder.target_dir(Path::new(&target_dir));
  }
  builder
}
