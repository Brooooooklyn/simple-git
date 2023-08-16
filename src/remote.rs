use std::mem;

use napi::{bindgen_prelude::*, Env, Error, JsFunction, Status};
use napi_derive::napi;

use crate::error::IntoNapiError;

#[napi]
/// An enumeration of the possible directions for a remote.
pub enum Direction {
  /// Data will be fetched (read) from this remote.
  Fetch,
  /// Data will be pushed (written) to this remote.
  Push,
}

impl From<Direction> for git2::Direction {
  fn from(value: Direction) -> Self {
    match value {
      Direction::Fetch => git2::Direction::Fetch,
      Direction::Push => git2::Direction::Push,
    }
  }
}

#[napi]
pub struct Remote {
  pub(crate) inner: SharedReference<crate::repo::Repository, git2::Remote<'static>>,
}

#[napi]
impl Remote {
  #[napi]
  /// Ensure the remote name is well-formed.
  pub fn is_valid_name(name: String) -> bool {
    git2::Remote::is_valid_name(&name)
  }

  #[napi]
  /// Get the remote's name.
  ///
  /// Returns `None` if this remote has not yet been named or if the name is
  /// not valid utf-8
  pub fn name(&self) -> Option<&str> {
    self.inner.name()
  }

  #[napi]
  /// Get the remote's url.
  ///
  /// Returns `None` if the url is not valid utf-8
  pub fn url(&self) -> Option<&str> {
    self.inner.url()
  }

  #[napi]
  /// Get the remote's pushurl.
  ///
  /// Returns `None` if the pushurl is not valid utf-8
  pub fn pushurl(&self) -> Option<&str> {
    self.inner.pushurl()
  }

  #[napi]
  /// Get the remote's default branch.
  ///
  /// The remote (or more exactly its transport) must have connected to the
  /// remote repository. This default branch is available as soon as the
  /// connection to the remote is initiated and it remains available after
  /// disconnecting.
  pub fn default_branch(&self) -> Result<String> {
    self
      .inner
      .default_branch()
      .convert("Get the default branch of Remote failed")
      .and_then(|b| {
        b.as_str().map(|name| name.to_owned()).ok_or_else(|| {
          Error::new(
            Status::GenericFailure,
            "Default branch name contains non-utf-8 characters".to_string(),
          )
        })
      })
  }

  #[napi]
  /// Open a connection to a remote.
  pub fn connect(&mut self, dir: Direction) -> Result<()> {
    self.inner.connect(dir.into()).convert_without_message()
  }

  #[napi]
  /// Check whether the remote is connected
  pub fn connected(&mut self) -> bool {
    self.inner.connected()
  }

  #[napi]
  /// Disconnect from the remote
  pub fn disconnect(&mut self) -> Result<()> {
    self.inner.disconnect().convert_without_message()
  }

  #[napi]
  /// Cancel the operation
  ///
  /// At certain points in its operation, the network code checks whether the
  /// operation has been cancelled and if so stops the operation.
  pub fn stop(&mut self) -> Result<()> {
    self.inner.stop().convert_without_message()
  }

  #[napi(ts_args_type = "refspecs: string[], cb?: (progress: Progress) => void")]
  pub fn fetch(
    &mut self,
    env: Env,
    refspecs: Vec<String>,
    fetch_callback: Option<JsFunction>,
  ) -> Result<()> {
    let mut options = git2::FetchOptions::default();
    let mut cbs = git2::RemoteCallbacks::default();
    cbs.credentials(|url, username_from_url, _| {
      if url.starts_with("http") || url.starts_with("https") {
        return git2::Cred::default();
      }
      if let Some(username_from_url) = username_from_url {
        git2::Cred::ssh_key(
          username_from_url,
          None,
          std::path::Path::new(&format!("{}/.ssh/id_rsa", std::env::var("HOME").unwrap())),
          None,
        )
      } else {
        git2::Cred::default()
      }
    });
    if let Some(callback) = fetch_callback {
      cbs.transfer_progress(move |progress| create_progress(&env, &progress, &callback).is_ok());
    }
    options.remote_callbacks(cbs);
    self
      .inner
      .fetch(refspecs.as_slice(), Some(&mut options), None)
      .convert_without_message()
  }
}

#[napi]
pub struct RemoteCallbacks {
  inner: git2::RemoteCallbacks<'static>,
}

#[napi]
impl RemoteCallbacks {
  #[napi(constructor)]
  #[allow(clippy::new_without_default)]
  pub fn new() -> RemoteCallbacks {
    RemoteCallbacks {
      inner: git2::RemoteCallbacks::new(),
    }
  }

  #[napi]
  pub fn transfer_progress(&mut self, env: Env, callback: JsFunction) {
    self
      .inner
      .transfer_progress(move |p| create_progress(&env, &p, &callback).is_ok());
  }

  #[napi(ts_args_type = "callback: (a: number, b: number, c: number) => void")]
  pub fn push_transfer_progress(&mut self, env: Env, callback: JsFunction) {
    self.inner.push_transfer_progress(move |a, b, c| {
      callback
        .call(
          None,
          &[
            env.create_uint32(a as u32).unwrap(),
            env.create_uint32(b as u32).unwrap(),
            env.create_uint32(c as u32).unwrap(),
          ],
        )
        .unwrap();
    });
  }
}

#[napi]
pub struct FetchOptions {
  inner: git2::FetchOptions<'static>,
}

#[napi]
impl FetchOptions {
  #[napi(constructor)]
  #[allow(clippy::new_without_default)]
  pub fn new() -> FetchOptions {
    FetchOptions {
      inner: git2::FetchOptions::new(),
    }
  }

  #[napi]
  pub fn remote_callback(&mut self, callback: &mut RemoteCallbacks) {
    let mut cbs = git2::RemoteCallbacks::default();
    mem::swap(&mut cbs, &mut callback.inner);
    self.inner.remote_callbacks(cbs);
  }
}

#[napi(object)]
pub struct Progress {
  pub total_objects: u32,
  pub indexed_objects: u32,
  pub received_objects: u32,
  pub local_objects: u32,
  pub total_deltas: u32,
  pub indexed_deltas: u32,
  pub received_bytes: u32,
}

fn create_progress(env: &Env, progress: &git2::Progress<'_>, callback: &JsFunction) -> Result<()> {
  let mut obj = env.create_object()?;
  obj.set("totalObjects", progress.total_objects() as u32)?;
  obj.set("indexedObjects", progress.indexed_objects() as u32)?;
  obj.set("receivedObjects", progress.received_objects() as u32)?;
  obj.set("localObjects", progress.local_objects() as u32)?;
  obj.set("totalDeltas", progress.total_deltas() as u32)?;
  obj.set("indexedDeltas", progress.indexed_deltas() as u32)?;
  obj.set("receivedBytes", progress.received_bytes() as u32)?;
  callback.call(None, &[obj])?;
  Ok(())
}
