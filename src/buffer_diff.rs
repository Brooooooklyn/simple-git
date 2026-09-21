use std::ffi::CString;
use std::path::Path;
use std::ptr;

use napi::bindgen_prelude::{Buffer, Uint8Array};
use napi_derive::napi;

use crate::diff::DiffOptions;
use crate::error::IntoNapiError;
use crate::repo::build_diff_options;

#[napi(object, use_nullable = true)]
/// A single line inside a `BufferDiffHunk`. `oldLineno`/`newLineno` are `null`
/// when the line does not exist on that side (added lines have no `oldLineno`,
/// deleted lines have no `newLineno`). `content` is the raw line bytes
/// (including any trailing newline, but NOT the leading `+`/`-`/space sigil —
/// that sigil is `origin`).
pub struct BufferDiffLine {
  /// One-character sigil classifying the line: `' '` context, `'+'` addition,
  /// `'-'` deletion, `'='`/`'>'`/`'<'` end-of-file markers, `'F'` file header,
  /// `'H'` hunk header, `'B'` binary marker.
  pub origin: String,
  pub old_lineno: Option<u32>,
  pub new_lineno: Option<u32>,
  pub content: Buffer,
}

#[napi(object)]
/// A single `@@`-delimited hunk of a buffer diff, with its header text and
/// eagerly materialized lines.
pub struct BufferDiffHunk {
  /// The hunk header text (e.g. `@@ -1,2 +1,3 @@`), as bytes decoded lossily.
  pub header: String,
  pub old_start: u32,
  pub old_lines: u32,
  pub new_start: u32,
  pub new_lines: u32,
  pub lines: Vec<BufferDiffLine>,
}

#[napi(object)]
/// Line counts for a buffer diff. `filesChanged` is `1` when the patch is
/// non-empty and `0` otherwise (`Patch` is a single-delta structure).
pub struct BufferDiffStats {
  pub additions: u32,
  pub deletions: u32,
  pub files_changed: u32,
}

#[napi(object)]
/// The fully materialized result of `diffBuffers`. Everything is copied out
/// eagerly because `git2::Patch` borrows the input buffers and cannot escape.
pub struct BufferDiffResult {
  /// The complete unified-diff bytes of the patch (empty buffer when the two
  /// sides are identical). Returned as raw bytes rather than a string so
  /// non-UTF-8 content is preserved exactly.
  pub patch: Buffer,
  pub hunks: Vec<BufferDiffHunk>,
  pub stats: BufferDiffStats,
}

fn opt_c_path(path: Option<&String>) -> crate::Result<Option<CString>> {
  path
    .map(|p| CString::new(Path::new(p).as_os_str().as_encoded_bytes()))
    .transpose()
    .map_err(|_| {
      napi::Error::new(
        crate::GitErrorCode::InvalidArg,
        "path contains an interior NUL byte".to_string(),
      )
    })
}

#[napi(
  ts_args_type = "oldBuffer: Uint8Array | null, oldPath: string | null, newBuffer: Uint8Array | null, newPath: string | null, options?: DiffOptions | null"
)]
/// Diff two raw in-memory buffers without a repository, like
/// `git diff --no-index`.
///
/// Pass `null` for a buffer to treat that side as absent (the delta is then
/// reported as an added or deleted file with a `/dev/null` header); passing
/// `null` for both yields an empty patch with zero hunks. An absent side is
/// distinct from an empty buffer: `diffBuffers(null, p, Buffer.alloc(0), p)`
/// reports a file creation. `oldPath`/`newPath` only label the output headers.
pub fn diff_buffers(
  old_buffer: Option<Uint8Array>,
  old_path: Option<String>,
  new_buffer: Option<Uint8Array>,
  new_path: Option<String>,
  options: Option<DiffOptions>,
) -> crate::Result<BufferDiffResult> {
  let mut diff_options = build_diff_options(options);
  let old_path = opt_c_path(old_path.as_ref())?;
  let new_path = opt_c_path(new_path.as_ref())?;

  // `git2::Patch::from_buffers` cannot express a NULL buffer (a `&[u8]` pointer
  // is never null), but NULL is what tells libgit2 a side is *absent* rather
  // than *empty* (GIT_DELTA_ADDED/DELETED instead of a modification from/to the
  // empty blob). Call `git_patch_from_buffers` directly so `None` maps to NULL,
  // then adopt the raw patch into `git2::Patch`.
  let mut patch: git2::Patch<'_> = unsafe {
    let mut raw_patch = ptr::null_mut();
    let code = libgit2_sys::git_patch_from_buffers(
      &mut raw_patch,
      old_buffer
        .as_deref()
        .map_or(ptr::null(), |b| b.as_ptr().cast()),
      old_buffer.as_deref().map_or(0, |b| b.len()),
      old_path.as_ref().map_or(ptr::null(), |p| p.as_ptr()),
      new_buffer
        .as_deref()
        .map_or(ptr::null(), |b| b.as_ptr().cast()),
      new_buffer.as_deref().map_or(0, |b| b.len()),
      new_path.as_ref().map_or(ptr::null(), |p| p.as_ptr()),
      diff_options.raw(),
    );
    if code < 0 {
      return Err(git2::Error::last_error(code)).convert_without_message();
    }
    git2::Binding::from_raw(raw_patch)
  };

  let patch_text: Buffer = patch.to_buf().convert_without_message()?.to_vec().into();

  let num_hunks = patch.num_hunks();
  let mut hunks = Vec::with_capacity(num_hunks);
  for hunk_idx in 0..num_hunks {
    let (hunk, line_count) = patch.hunk(hunk_idx).convert_without_message()?;
    let mut lines = Vec::with_capacity(line_count);
    for line_idx in 0..line_count {
      let line = patch
        .line_in_hunk(hunk_idx, line_idx)
        .convert_without_message()?;
      lines.push(BufferDiffLine {
        origin: line.origin().to_string(),
        old_lineno: line.old_lineno(),
        new_lineno: line.new_lineno(),
        content: line.content().to_vec().into(),
      });
    }
    hunks.push(BufferDiffHunk {
      header: String::from_utf8_lossy(hunk.header()).into_owned(),
      old_start: hunk.old_start(),
      old_lines: hunk.old_lines(),
      new_start: hunk.new_start(),
      new_lines: hunk.new_lines(),
      lines,
    });
  }

  let (_context, additions, deletions) = patch.line_stats().convert_without_message()?;
  // A hunk-less delta can still print file headers (e.g. binary content, or an
  // added/deleted empty file), so the patch text decides "did anything change".
  let files_changed = u32::from(num_hunks > 0 || !patch_text.is_empty());
  Ok(BufferDiffResult {
    patch: patch_text,
    hunks,
    stats: BufferDiffStats {
      additions: additions as u32,
      deletions: deletions as u32,
      files_changed,
    },
  })
}
