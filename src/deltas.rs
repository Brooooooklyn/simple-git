use napi::{JsString, bindgen_prelude::*};
use napi_derive::napi;

use crate::util::path_to_javascript_string;

#[napi]
#[repr(u32)]
pub enum DiffFlags {
  /// File(s) treated as binary data.
  /// 1 << 0
  Binary = 1,
  /// File(s) treated as text data.
  /// 1 << 1
  NotBinary = 2,
  /// `id` value is known correct.
  /// 1 << 2
  ValidId = 4,
  /// File exists at this side of the delta.
  /// 1 << 3
  Exists = 8,
}

impl From<DiffFlags> for git2::DiffFlags {
  fn from(value: DiffFlags) -> Self {
    match value {
      DiffFlags::Binary => git2::DiffFlags::BINARY,
      DiffFlags::NotBinary => git2::DiffFlags::NOT_BINARY,
      DiffFlags::ValidId => git2::DiffFlags::VALID_ID,
      DiffFlags::Exists => git2::DiffFlags::EXISTS,
    }
  }
}

impl From<git2::DiffFlags> for DiffFlags {
  fn from(value: git2::DiffFlags) -> Self {
    match value {
      git2::DiffFlags::BINARY => DiffFlags::Binary,
      git2::DiffFlags::NOT_BINARY => DiffFlags::NotBinary,
      git2::DiffFlags::VALID_ID => DiffFlags::ValidId,
      git2::DiffFlags::EXISTS => DiffFlags::Exists,
      _ => DiffFlags::Binary,
    }
  }
}

#[napi]
/// Valid modes for index and tree entries.
pub enum FileMode {
  /// Unreadable
  Unreadable,
  /// Tree
  Tree,
  /// Blob
  Blob,
  /// Group writable blob. Obsolete mode kept for compatibility reasons
  BlobGroupWritable,
  /// Blob executable
  BlobExecutable,
  /// Link
  Link,
  /// Commit
  Commit,
}

impl From<git2::FileMode> for FileMode {
  fn from(value: git2::FileMode) -> Self {
    match value {
      git2::FileMode::Unreadable => FileMode::Unreadable,
      git2::FileMode::Tree => FileMode::Tree,
      git2::FileMode::Blob => FileMode::Blob,
      git2::FileMode::BlobGroupWritable => FileMode::BlobGroupWritable,
      git2::FileMode::BlobExecutable => FileMode::BlobExecutable,
      git2::FileMode::Link => FileMode::Link,
      git2::FileMode::Commit => FileMode::Commit,
    }
  }
}

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
  /// Returns the flags on the delta.
  ///
  /// For more information, see `DiffFlags`'s documentation.
  pub fn flags(&self) -> DiffFlags {
    self.inner.flags().into()
  }

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
  /// Returns the Oid of this item.
  ///
  /// If this entry represents an absent side of a diff (e.g. the `old_file`
  /// of a `Added` delta), then the oid returned will be zeroes.
  pub fn id(&self) -> String {
    self.inner.id().to_string()
  }

  #[napi]
  /// Returns the path, in bytes, of the entry relative to the working
  /// directory of the repository.
  pub fn path<'env>(&'env self, env: &'env Env) -> Option<JsString<'env>> {
    self
      .inner
      .path()
      .and_then(|p| path_to_javascript_string(env, p).ok())
  }

  #[napi]
  /// Returns the size of this entry, in bytes
  pub fn size(&self) -> u64 {
    self.inner.size()
  }

  #[napi]
  /// Returns `true` if file(s) are treated as binary data.
  pub fn is_binary(&self) -> bool {
    self.inner.is_binary()
  }

  #[napi]
  /// Returns `true` if file(s) are treated as text data.
  pub fn is_not_binary(&self) -> bool {
    self.inner.is_not_binary()
  }

  #[napi]
  /// Returns `true` if `id` value is known correct.
  pub fn is_valid_id(&self) -> bool {
    self.inner.is_valid_id()
  }

  #[napi]
  /// Returns `true` if file exists at this side of the delta.
  pub fn exists(&self) -> bool {
    self.inner.exists()
  }

  #[napi]
  /// Returns file mode.
  pub fn mode(&self) -> FileMode {
    self.inner.mode().into()
  }
}
