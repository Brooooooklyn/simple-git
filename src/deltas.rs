use napi::bindgen_prelude::{Generator, SharedReference, ToNapiValue};
use napi_derive::napi;

#[napi(iterator)]
/// An iterator over the diffs in a delta
pub struct Deltas {
  pub(crate) inner: SharedReference<crate::diff::Diff, git2::Deltas<'static>>,
}

#[napi]
impl Generator for Deltas {
  type Yield = DiffDelta;
  type Next = ();
  type Return = ();

  fn next(&mut self, _value: Option<()>) -> Option<Self::Yield> {
    self.inner.next().map(|delta| DiffDelta { inner: delta })
  }
}

#[napi]
pub struct DiffDelta {
  pub(crate) inner: git2::DiffDelta<'static>,
}

#[napi]
impl DiffDelta {
  #[napi]
  /// Returns the number of files in this delta.
  pub fn num_files(&self) -> u32 {
    self.inner.nfiles() as u32
  }

  #[napi]
  /// Returns the status of this entry
  pub fn status(&self) -> Delta {
    self.inner.status().into()
  }

  #[napi]
  /// Return the file which represents the "from" side of the diff.
  ///
  /// What side this means depends on the function that was used to generate
  /// the diff and will be documented on the function itself.
  pub fn old_file(&self) -> DiffFile {
    DiffFile {
      inner: self.inner.old_file(),
    }
  }

  #[napi]
  /// Return the file which represents the "to" side of the diff.
  ///
  /// What side this means depends on the function that was used to generate
  /// the diff and will be documented on the function itself.
  pub fn new_file(&self) -> DiffFile {
    DiffFile {
      inner: self.inner.new_file(),
    }
  }
}

#[napi]
pub enum Delta {
  /// No changes
  Unmodified,
  /// Entry does not exist in old version
  Added,
  /// Entry does not exist in new version
  Deleted,
  /// Entry content changed between old and new
  Modified,
  /// Entry was renamed between old and new
  Renamed,
  /// Entry was copied from another old entry
  Copied,
  /// Entry is ignored item in workdir
  Ignored,
  /// Entry is untracked item in workdir
  Untracked,
  /// Type of entry changed between old and new
  Typechange,
  /// Entry is unreadable
  Unreadable,
  /// Entry in the index is conflicted
  Conflicted,
}

impl From<git2::Delta> for Delta {
  fn from(delta: git2::Delta) -> Self {
    match delta {
      git2::Delta::Unmodified => Delta::Unmodified,
      git2::Delta::Added => Delta::Added,
      git2::Delta::Deleted => Delta::Deleted,
      git2::Delta::Modified => Delta::Modified,
      git2::Delta::Renamed => Delta::Renamed,
      git2::Delta::Copied => Delta::Copied,
      git2::Delta::Ignored => Delta::Ignored,
      git2::Delta::Untracked => Delta::Untracked,
      git2::Delta::Typechange => Delta::Typechange,
      git2::Delta::Unreadable => Delta::Unreadable,
      git2::Delta::Conflicted => Delta::Conflicted,
    }
  }
}

#[napi]
pub struct DiffFile {
  pub(crate) inner: git2::DiffFile<'static>,
}

#[napi]
impl DiffFile {
  #[napi]
  /// Returns the path, in bytes, of the entry relative to the working
  /// directory of the repository.
  pub fn path(&self) -> Option<&str> {
    self
      .inner
      .path_bytes()
      .and_then(|bytes| std::str::from_utf8(bytes).ok())
  }
}
