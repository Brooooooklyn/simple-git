use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub struct Reference {
  pub(crate) repo: String,
}

macro_rules! get_inner {
  ($repo:expr) => {
    crate::repo::REPO_CACHE
      .0
      .get($repo)
      .unwrap()
      .head()
      .map_err(|err| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to get git head {}", err),
        )
      })
  };
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
    Ok(get_inner!(&self.repo)?.is_branch())
  }

  #[napi]
  /// Check if a reference is a note.
  pub fn is_note(&self) -> Result<bool> {
    Ok(get_inner!(&self.repo)?.is_note())
  }

  #[napi]
  /// Check if a reference is a remote tracking branch
  pub fn is_remote(&self) -> Result<bool> {
    Ok(get_inner!(&self.repo)?.is_remote())
  }

  #[napi]
  /// Check if a reference is a tag
  pub fn is_tag(&self) -> Result<bool> {
    Ok(get_inner!(&self.repo)?.is_tag())
  }

  #[napi]
  pub fn kind(&self) -> Result<ReferenceType> {
    match get_inner!(&self.repo)?.kind() {
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
    get_inner!(&self.repo).ok()?.name().map(|s| s.to_string())
  }

  #[napi]
  /// Get the full shorthand of a reference.
  ///
  /// This will transform the reference name into a name "human-readable"
  /// version. If no shortname is appropriate, it will return the full name.
  ///
  /// Returns `None` if the shorthand is not valid utf-8.
  pub fn shorthand(&self) -> Option<String> {
    get_inner!(&self.repo)
      .ok()?
      .shorthand()
      .map(|s| s.to_string())
  }

  #[napi]
  /// Get the OID pointed to by a direct reference.
  ///
  /// Only available if the reference is direct (i.e. an object id reference,
  /// not a symbolic one).
  pub fn target(&self) -> Option<String> {
    get_inner!(&self.repo)
      .ok()?
      .target()
      .map(|oid| oid.to_string())
  }

  #[napi]
  /// Return the peeled OID target of this reference.
  ///
  /// This peeled OID only applies to direct references that point to a hard
  /// Tag object: it is the result of peeling such Tag.
  pub fn target_peel(&self) -> Option<String> {
    get_inner!(&self.repo)
      .ok()?
      .target_peel()
      .map(|oid| oid.to_string())
  }

  #[napi]
  /// Get full name to the reference pointed to by a symbolic reference.
  ///
  /// May return `None` if the reference is either not symbolic or not a
  /// valid utf-8 string.
  pub fn symbolic_target(&self) -> Option<String> {
    get_inner!(&self.repo)
      .ok()?
      .symbolic_target()
      .map(|s| s.to_owned())
  }
}
