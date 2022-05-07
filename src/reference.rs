use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::error::IntoNapiError;
use crate::tree::{Tree, TreeParent};

#[napi]
pub struct Reference {
  pub(crate) inner:
    napi::bindgen_prelude::SharedReference<crate::repo::Repository, git2::Reference<'static>>,
}

#[napi]
#[derive(PartialEq, Eq, Debug)]
/// An enumeration of all possible kinds of references.
pub enum ReferenceType {
  /// A reference which points at an object id.
  Direct,

  /// A reference which points at another reference.
  Symbolic,

  Unknown,
}

#[napi]
impl Reference {
  #[napi]
  /// Ensure the reference name is well-formed.
  ///
  /// Validation is performed as if [`ReferenceFormat::ALLOW_ONELEVEL`]
  /// was given to [`Reference.normalize_name`]. No normalization is
  /// performed, however.
  ///
  /// ```ts
  /// import { Reference } from '@napi-rs/simple-git'
  ///
  /// console.assert(Reference.is_valid_name("HEAD"));
  /// console.assert(Reference.is_valid_name("refs/heads/main"));
  ///
  /// // But:
  /// console.assert(!Reference.is_valid_name("main"));
  /// console.assert(!Reference.is_valid_name("refs/heads/*"));
  /// console.assert(!Reference.is_valid_name("foo//bar"));
  /// ```
  pub fn is_valid_name(name: String) -> bool {
    git2::Reference::is_valid_name(&name)
  }

  #[napi]
  /// Check if a reference is a local branch.
  pub fn is_branch(&self) -> Result<bool> {
    Ok(self.inner.is_branch())
  }

  #[napi]
  /// Check if a reference is a note.
  pub fn is_note(&self) -> Result<bool> {
    Ok(self.inner.is_note())
  }

  #[napi]
  /// Check if a reference is a remote tracking branch
  pub fn is_remote(&self) -> Result<bool> {
    Ok(self.inner.is_remote())
  }

  #[napi]
  /// Check if a reference is a tag
  pub fn is_tag(&self) -> Result<bool> {
    Ok(self.inner.is_tag())
  }

  #[napi]
  pub fn kind(&self) -> Result<ReferenceType> {
    match self.inner.kind() {
      Some(git2::ReferenceType::Symbolic) => Ok(ReferenceType::Symbolic),
      Some(git2::ReferenceType::Direct) => Ok(ReferenceType::Direct),
      _ => Ok(ReferenceType::Unknown),
    }
  }

  #[napi]
  /// Get the full name of a reference.
  ///
  /// Returns `None` if the name is not valid utf-8.
  pub fn name(&self) -> Option<String> {
    self.inner.name().map(|s| s.to_string())
  }

  #[napi]
  /// Get the full shorthand of a reference.
  ///
  /// This will transform the reference name into a name "human-readable"
  /// version. If no shortname is appropriate, it will return the full name.
  ///
  /// Returns `None` if the shorthand is not valid utf-8.
  pub fn shorthand(&self) -> Option<String> {
    self.inner.shorthand().map(|s| s.to_string())
  }

  #[napi]
  /// Get the OID pointed to by a direct reference.
  ///
  /// Only available if the reference is direct (i.e. an object id reference,
  /// not a symbolic one).
  pub fn target(&self) -> Option<String> {
    self.inner.target().map(|oid| oid.to_string())
  }

  #[napi]
  /// Return the peeled OID target of this reference.
  ///
  /// This peeled OID only applies to direct references that point to a hard
  /// Tag object: it is the result of peeling such Tag.
  pub fn target_peel(&self) -> Option<String> {
    self.inner.target_peel().map(|oid| oid.to_string())
  }

  #[napi]
  /// Peel a reference to a tree
  ///
  /// This method recursively peels the reference until it reaches
  /// a tree.
  pub fn peel_to_tree(
    &self,
    env: Env,
    self_ref: napi::bindgen_prelude::Reference<Reference>,
  ) -> Result<Tree> {
    Ok(Tree {
      inner: TreeParent::Reference(self_ref.share_with(env, |reference| {
        reference.inner.peel_to_tree().convert_without_message()
      })?),
    })
  }

  #[napi]
  /// Get full name to the reference pointed to by a symbolic reference.
  ///
  /// May return `None` if the reference is either not symbolic or not a
  /// valid utf-8 string.
  pub fn symbolic_target(&self) -> Option<String> {
    self.inner.symbolic_target().map(|s| s.to_owned())
  }

  #[napi]
  /// Resolve a symbolic reference to a direct reference.
  ///
  /// This method iteratively peels a symbolic reference until it resolves to
  /// a direct reference to an OID.
  ///
  /// If a direct reference is passed as an argument, a copy of that
  /// reference is returned.
  pub fn resolve(&self, env: Env) -> Result<Reference> {
    let shared = self
      .inner
      .clone(env)?
      .share_with(env, |r| r.resolve().convert_without_message())?;
    Ok(Self { inner: shared })
  }

  #[napi]
  /// Rename an existing reference.
  ///
  /// This method works for both direct and symbolic references.
  ///
  /// If the force flag is not enabled, and there's already a reference with
  /// the given name, the renaming will fail.
  pub fn rename(
    &mut self,
    env: Env,
    new_name: String,
    force: bool,
    msg: String,
  ) -> Result<Reference> {
    let inner = self.inner.clone(env)?.share_with(env, |r| {
      r.rename(&new_name, force, &msg).convert_without_message()
    })?;
    Ok(Self { inner })
  }
}
