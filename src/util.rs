use std::path::Path;

use napi::{Env, JsString, Result};

pub(crate) fn path_to_javascript_string(env: &Env, p: &Path) -> Result<JsString> {
  #[cfg(unix)]
  {
    use std::borrow::Borrow;

    let path = p.to_string_lossy();
    env.create_string(path.borrow())
  }
  #[cfg(windows)]
  {
    use std::os::windows::ffi::OsStrExt;
    let path_buf = p.as_os_str().encode_wide().collect::<Vec<u16>>();
    env.create_string_utf16(path_buf.as_slice())
  }
}
