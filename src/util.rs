use std::path::Path;

use napi::{Env, JsString, Result};

pub(crate) fn path_to_javascript_string<'env>(
  env: &'env Env,
  p: &'env Path,
) -> Result<JsString<'env>> {
  #[cfg(unix)]
  {
    let path = p.to_string_lossy();
    env.create_string(path.as_ref())
  }
  #[cfg(windows)]
  {
    use std::os::windows::ffi::OsStrExt;
    let path_buf = p.as_os_str().encode_wide().collect::<Vec<u16>>();
    env.create_string_utf16(path_buf.as_slice())
  }
}
