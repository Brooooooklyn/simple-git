use std::{mem, path::Path};

use git2::{ErrorClass, ErrorCode};
use napi::bindgen_prelude::*;
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
/// Configuration for how pruning is done on a fetch
pub enum FetchPrune {
  /// Use the setting from the configuration
  Unspecified,
  /// Force pruning on
  On,
  /// Force pruning off
  Off,
}

impl From<FetchPrune> for git2::FetchPrune {
  fn from(value: FetchPrune) -> Self {
    match value {
      FetchPrune::Unspecified => git2::FetchPrune::Unspecified,
      FetchPrune::On => git2::FetchPrune::On,
      FetchPrune::Off => git2::FetchPrune::Off,
    }
  }
}

#[napi]
/// Automatic tag following options.
pub enum AutotagOption {
  /// Use the setting from the remote's configuration
  Unspecified,
  /// Ask the server for tags pointing to objects we're already downloading
  Auto,
  /// Don't ask for any tags beyond the refspecs
  None,
  /// Ask for all the tags
  All,
}

impl From<AutotagOption> for git2::AutotagOption {
  fn from(value: AutotagOption) -> Self {
    match value {
      AutotagOption::Unspecified => git2::AutotagOption::Unspecified,
      AutotagOption::Auto => git2::AutotagOption::Auto,
      AutotagOption::None => git2::AutotagOption::None,
      AutotagOption::All => git2::AutotagOption::All,
    }
  }
}

#[napi]
/// Remote redirection settings; whether redirects to another host are
/// permitted.
///
/// By default, git will follow a redirect on the initial request
/// (`/info/refs`), but not subsequent requests.
pub enum RemoteRedirect {
  /// Do not follow any off-site redirects at any stage of the fetch or push.
  None,
  /// Allow off-site redirects only upon the initial request. This is the
  /// default.
  Initial,
  /// Allow redirects at any stage in the fetch or push.
  All,
}

impl From<RemoteRedirect> for git2::RemoteRedirect {
  fn from(value: RemoteRedirect) -> Self {
    match value {
      RemoteRedirect::None => git2::RemoteRedirect::None,
      RemoteRedirect::Initial => git2::RemoteRedirect::Initial,
      RemoteRedirect::All => git2::RemoteRedirect::All,
    }
  }
}

#[napi]
/// Types of credentials that can be requested by a credential callback.
pub enum CredentialType {
  /// 1 << 0
  UserPassPlaintext = 1,
  /// 1 << 1
  SshKey = 2,
  /// 1 << 6
  SshMemory = 64,
  /// 1 << 2
  SshCustom = 4,
  /// 1 << 3
  Default = 8,
  /// 1 << 4
  SshInteractive = 16,
  /// 1 << 5
  Username = 32,
}

impl From<CredentialType> for git2::CredentialType {
  fn from(value: CredentialType) -> Self {
    match value {
      CredentialType::UserPassPlaintext => git2::CredentialType::USER_PASS_PLAINTEXT,
      CredentialType::SshKey => git2::CredentialType::SSH_KEY,
      CredentialType::SshMemory => git2::CredentialType::SSH_MEMORY,
      CredentialType::SshCustom => git2::CredentialType::SSH_CUSTOM,
      CredentialType::Default => git2::CredentialType::DEFAULT,
      CredentialType::SshInteractive => git2::CredentialType::SSH_INTERACTIVE,
      CredentialType::Username => git2::CredentialType::USERNAME,
    }
  }
}

#[napi(object)]
pub struct CredInfo {
  /// Raw `CredentialType` bitset of the credential types the server will
  /// accept. OR-able; test bits with `credTypeContains`.
  pub cred_type: u32,
  pub url: String,
  pub username: String,
}

#[napi]
#[repr(u32)]
/// OR-able flags for `Remote.updateTips`. Each discriminant is the real libgit2
/// `GIT_REMOTE_UPDATE_*` bit, so they can be combined with `|`.
pub enum RemoteUpdateFlags {
  UpdateFetchHead = 1,
  ReportUnchanged = 2,
}

#[napi]
pub struct Remote {
  pub(crate) inner: SharedReference<crate::repo::Repository, git2::Remote<'static>>,
  /// Gitdir of the owning repository (`Repository::path()`), captured on the JS
  /// thread. `git2::Remote` exposes no owning-repo/path accessor, so the async
  /// `fetch`/`push` workers reopen the repository from this path and re-resolve
  /// the remote there instead of moving the JS-visible handle off-thread. Not a
  /// `#[napi]` field, so it is invisible to the JS surface.
  pub(crate) repo_path: String,
  /// The owning repository's active namespace (`Repository::namespace()`),
  /// captured at remote-CREATION time on the JS thread. The namespace is
  /// in-memory per-handle state on the parent `Repository`, and a worker that
  /// reopens the repo from `repo_path` would otherwise resolve/write the
  /// NON-namespaced refs. The async `fetch`/`push` workers re-apply this before
  /// resolving the remote so ref updates land in `refs/namespaces/<ns>/…`.
  /// CAVEAT: it is captured when this `Remote` is constructed; the (rare) case
  /// of `setNamespace` changing between `findRemote()` and the async call uses
  /// the creation-time namespace. The common case (namespace set before/at
  /// remote creation) is exact. This is acceptable because there is no live
  /// parent-repo handle available inside the `Remote` at async-call time. Not a
  /// `#[napi]` field, so it is invisible to the JS surface.
  pub(crate) namespace: Option<String>,
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
    self.inner.name().ok().flatten()
  }

  #[napi]
  /// Get the remote's url.
  ///
  /// Returns `None` if the url is not valid utf-8
  pub fn url(&self) -> Option<&str> {
    self.inner.url().ok()
  }

  #[napi]
  /// Get the remote's pushurl.
  ///
  /// Returns `None` if the pushurl is not valid utf-8
  pub fn pushurl(&self) -> Option<&str> {
    self.inner.pushurl().ok().flatten()
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
        b.as_str().ok().map(|name| name.to_owned()).ok_or_else(|| {
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

  #[napi]
  /// Download new data and update tips
  ///
  /// Convenience function to connect to a remote, download the data,
  /// disconnect and update the remote-tracking branches.
  ///
  pub fn fetch(
    &mut self,
    refspecs: Vec<String>,
    fetch_options: Option<&mut FetchOptions>,
  ) -> Result<()> {
    let mut default_fetch_options = git2::FetchOptions::default();
    let mut options = match fetch_options {
      Some(o) => {
        if o.used {
          return Err(Error::new(
            Status::GenericFailure,
            "FetchOptions can only be used once".to_string(),
          ));
        }
        std::mem::swap(&mut o.inner, &mut default_fetch_options);
        o.used = true;
        default_fetch_options
      }
      None => git2::FetchOptions::default(),
    };
    self
      .inner
      .fetch(refspecs.as_slice(), Some(&mut options), None)
      .convert_without_message()
  }

  #[napi]
  /// Perform a push.
  ///
  /// If `refspecs` is empty the configured push refspecs are used. Delete a
  /// remote ref by pushing `":refs/heads/branch"`. To detect per-ref server
  /// rejections, set a `pushUpdateReference` callback on the `RemoteCallbacks`.
  pub fn push(
    &mut self,
    refspecs: Vec<String>,
    push_options: Option<&mut PushOptions>,
  ) -> Result<()> {
    let mut default_push_options = git2::PushOptions::default();
    let mut options = match push_options {
      Some(o) => {
        if o.used {
          return Err(Error::new(
            Status::GenericFailure,
            "PushOptions can only be used once".to_string(),
          ));
        }
        std::mem::swap(&mut o.inner, &mut default_push_options);
        o.used = true;
        default_push_options
      }
      None => git2::PushOptions::default(),
    };
    self
      .inner
      .push(refspecs.as_slice(), Some(&mut options))
      .convert_without_message()
  }

  #[napi(ts_return_type = "Promise<void>")]
  /// Asynchronous variant of `fetch`, performed off the main thread.
  ///
  /// `fetchOptions` may carry data-only settings (depth, prune, proxy url,
  /// headers, ...). It must NOT carry `RemoteCallbacks`: those hold JS-backed
  /// callbacks bound to the main JS thread and cannot be invoked safely from a
  /// worker thread. If callbacks are required, use the synchronous `fetch`.
  ///
  /// Resolves against a URL/refspec snapshot captured from this loaded
  /// `Remote` at call time, not live on-disk config — a later
  /// `remoteSetUrl`/`remoteAddFetch`/`remoteDelete` on the same name does not
  /// affect an already-scheduled fetch, matching the synchronous `fetch()`
  /// contract ("no loaded remote instances will be affected").
  ///
  /// Safety: do not use the same `Remote` from the main thread while this async
  /// operation is pending; the underlying git2 handle is not `Sync`.
  pub fn fetch_async(
    &self,
    refspecs: Vec<String>,
    fetch_options: Option<&mut FetchOptions>,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<RemoteFetchTask>> {
    let options = match fetch_options {
      Some(o) => {
        if o.used {
          return Err(Error::new(
            Status::GenericFailure,
            "FetchOptions can only be used once".to_string(),
          ));
        }
        if o.has_remote_callbacks {
          return Err(Error::new(
            Status::GenericFailure,
            "fetchAsync does not support RemoteCallbacks; use the synchronous fetch() instead"
              .to_string(),
          ));
        }
        let mut taken = git2::FetchOptions::default();
        mem::swap(&mut o.inner, &mut taken);
        o.used = true;
        Some(taken)
      }
      None => None,
    };
    let remote_url = self.inner.url().map(|s| s.to_owned()).map_err(|_| {
      Error::new(
        Status::GenericFailure,
        "Remote has no valid UTF-8 fetch URL".to_string(),
      )
    })?;
    let refspecs = if refspecs.is_empty() {
      self
        .inner
        .fetch_refspecs()
        .convert("Failed to read remote fetch refspecs")?
        .iter()
        .filter_map(|r| r.ok().flatten().map(|s| s.to_owned()))
        .collect()
    } else {
      refspecs
    };
    Ok(AsyncTask::with_optional_signal(
      RemoteFetchTask {
        repo_path: self.repo_path.clone(),
        namespace: self.namespace.clone(),
        remote_url,
        refspecs,
        options,
      },
      signal,
    ))
  }

  #[napi(ts_return_type = "Promise<void>")]
  /// Asynchronous variant of `push`, performed off the main thread.
  ///
  /// `pushOptions` may carry data-only settings (packbuilder parallelism,
  /// proxy url, headers, ...). It must NOT carry `RemoteCallbacks`: those hold
  /// JS-backed callbacks bound to the main JS thread and cannot be invoked
  /// safely from a worker thread. If callbacks (e.g. `pushUpdateReference`) are
  /// required, use the synchronous `push`.
  ///
  /// Resolves against a URL/refspec snapshot captured from this loaded
  /// `Remote` at call time (using the configured `pushurl` when set, else
  /// `url`), not live on-disk config — a later
  /// `remoteSetUrl`/`remoteAddFetch`/`remoteDelete` on the same name does not
  /// affect an already-scheduled push, matching the synchronous `push()`
  /// contract ("no loaded remote instances will be affected").
  ///
  /// Safety: do not use the same `Remote` from the main thread while this async
  /// operation is pending; the underlying git2 handle is not `Sync`.
  pub fn push_async(
    &self,
    refspecs: Vec<String>,
    push_options: Option<&mut PushOptions>,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<RemotePushTask>> {
    let options = match push_options {
      Some(o) => {
        if o.used {
          return Err(Error::new(
            Status::GenericFailure,
            "PushOptions can only be used once".to_string(),
          ));
        }
        if o.has_remote_callbacks {
          return Err(Error::new(
            Status::GenericFailure,
            "pushAsync does not support RemoteCallbacks; use the synchronous push() instead"
              .to_string(),
          ));
        }
        let mut taken = git2::PushOptions::default();
        mem::swap(&mut o.inner, &mut taken);
        o.used = true;
        Some(taken)
      }
      None => None,
    };
    let remote_url = self
      .inner
      .pushurl()
      .convert("Failed to read remote pushurl")?
      .map(|s| s.to_owned())
      .or_else(|| self.inner.url().ok().map(|s| s.to_owned()))
      .ok_or_else(|| {
        Error::new(
          Status::GenericFailure,
          "Remote has no valid UTF-8 push URL".to_string(),
        )
      })?;
    let refspecs = if refspecs.is_empty() {
      self
        .inner
        .push_refspecs()
        .convert("Failed to read remote push refspecs")?
        .iter()
        .filter_map(|r| r.ok().flatten().map(|s| s.to_owned()))
        .collect()
    } else {
      refspecs
    };
    Ok(AsyncTask::with_optional_signal(
      RemotePushTask {
        repo_path: self.repo_path.clone(),
        namespace: self.namespace.clone(),
        remote_url,
        refspecs,
        options,
      },
      signal,
    ))
  }

  #[napi]
  /// Update the tips to the new state
  ///
  /// `update_fetchhead` is a raw bitset of `RemoteUpdateFlags` OR-ed together
  /// (e.g. `RemoteUpdateFlags.UpdateFetchHead`). Unknown bits are ignored.
  pub fn update_tips(
    &mut self,
    update_fetchhead: u32,
    download_tags: AutotagOption,
    mut callbacks: Option<&mut RemoteCallbacks>,
    msg: Option<String>,
  ) -> Result<()> {
    let callbacks = callbacks.as_mut().map(|o| &mut o.inner);
    self
      .inner
      .update_tips(
        callbacks,
        git2::RemoteUpdateFlags::from_bits_truncate(update_fetchhead),
        download_tags.into(),
        msg.as_deref(),
      )
      .convert_without_message()
  }
}

pub struct RemoteFetchTask {
  repo_path: String,
  /// Active namespace of the owning repo, captured at remote-creation time (see
  /// `Remote::namespace`). Re-applied to the reopened worker handle before the
  /// remote is resolved so fetched refs land in `refs/namespaces/<ns>/…`.
  namespace: Option<String>,
  /// The remote's fetch URL, captured from the loaded `Remote` at
  /// `fetchAsync` call time. The worker reconstructs an anonymous remote from
  /// this URL rather than re-resolving by name against on-disk config, so a
  /// concurrent `remoteSetUrl`/`remoteAddFetch`/`remoteDelete` on the same
  /// name does not affect an in-flight fetch — matching the snapshot
  /// semantics of the synchronous `fetch()`.
  remote_url: String,
  refspecs: Vec<String>,
  options: Option<git2::FetchOptions<'static>>,
}

// SAFETY: every field is `Send` EXCEPT `git2::FetchOptions`, which is `!Send`
// only because it holds a `Vec<*const c_char>` of raw header pointers. It is
// *moved* into the task and only ever touched inside `compute()` on a single
// worker thread. Crucially, the stored `FetchOptions` is only ever constructed
// when the source `FetchOptions` carries NO `RemoteCallbacks` (guarded by
// `has_remote_callbacks` in `fetch_async`), so it holds only plain owned data
// (depth/prune/proxy url/headers) with no JS `Env` or threadsafe function
// captured — nothing unsound to use off the JS thread. The repository handle is
// REOPENED from the owned `repo_path` inside `compute()`, and the remote is
// reconstructed there from the captured URL snapshot via `remote_anonymous`
// (never re-resolved by name against on-disk config), so the task never
// aliases the JS-visible handle. No aliasing, no concurrent access: the move
// is sound.
unsafe impl Send for RemoteFetchTask {}

#[napi]
impl Task for RemoteFetchTask {
  type Output = ();
  type JsValue = ();

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let repo = git2::Repository::open(&self.repo_path)
      .convert(format!("Failed to open git repo: [{}]", self.repo_path))?;
    // Restore the parent repo's namespace before resolving the remote so the
    // reopened worker handle resolves/updates the namespaced refs, matching
    // sync `fetch`.
    if let Some(ns) = &self.namespace {
      repo
        .set_namespace(ns)
        .convert("Failed to restore repository namespace")?;
    }
    let mut remote = repo
      .remote_anonymous(&self.remote_url)
      .convert("Failed to resolve remote")?;
    let mut options = self.options.take().unwrap_or_default();
    remote
      .fetch(self.refspecs.as_slice(), Some(&mut options), None)
      .convert_without_message()
  }

  fn resolve(&mut self, _env: napi::Env, _output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(())
  }
}

pub struct RemotePushTask {
  repo_path: String,
  /// Active namespace of the owning repo, captured at remote-creation time (see
  /// `Remote::namespace`). Re-applied to the reopened worker handle before the
  /// remote is resolved so pushed refs resolve under `refs/namespaces/<ns>/…`.
  namespace: Option<String>,
  /// The remote's effective push URL (configured `pushurl` if set, else
  /// `url`), captured from the loaded `Remote` at `pushAsync` call time. The
  /// worker reconstructs an anonymous remote from this URL rather than
  /// re-resolving by name against on-disk config, so a concurrent
  /// `remoteSetUrl`/`remoteAddFetch`/`remoteDelete` on the same name does not
  /// affect an in-flight push — matching the snapshot semantics of the
  /// synchronous `push()`. Using this URL directly on an anonymous remote
  /// reproduces the push target exactly: an anonymous remote never has an
  /// in-memory pushurl override, so libgit2 falls back to its single `url` —
  /// which is already the effective target we captured.
  remote_url: String,
  refspecs: Vec<String>,
  options: Option<git2::PushOptions<'static>>,
}

// SAFETY: identical reasoning to `RemoteFetchTask`. Every field is `Send`
// EXCEPT `git2::PushOptions`, which is `!Send` only for its `Vec<*const c_char>`
// raw header pointers. The stored `PushOptions` is only ever built when the
// source `PushOptions` carries NO `RemoteCallbacks` (guarded by
// `has_remote_callbacks` in `push_async`), so it captures no JS `Env`/threadsafe
// function. The repository is REOPENED from the owned `repo_path` inside
// `compute()`, and the remote is reconstructed there from the captured URL
// snapshot via `remote_anonymous` (never re-resolved by name against on-disk
// config), all on a single worker thread — never aliasing or concurrently
// accessing the JS-visible handle.
unsafe impl Send for RemotePushTask {}

#[napi]
impl Task for RemotePushTask {
  type Output = ();
  type JsValue = ();

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let repo = git2::Repository::open(&self.repo_path)
      .convert(format!("Failed to open git repo: [{}]", self.repo_path))?;
    // Restore the parent repo's namespace before resolving the remote so the
    // reopened worker handle resolves the namespaced refs, matching sync `push`.
    if let Some(ns) = &self.namespace {
      repo
        .set_namespace(ns)
        .convert("Failed to restore repository namespace")?;
    }
    let mut remote = repo
      .remote_anonymous(&self.remote_url)
      .convert("Failed to resolve remote")?;
    let mut options = self.options.take().unwrap_or_default();
    remote
      .push(self.refspecs.as_slice(), Some(&mut options))
      .convert_without_message()
  }

  fn resolve(&mut self, _env: napi::Env, _output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(())
  }
}

#[napi]
pub struct RemoteCallbacks {
  inner: git2::RemoteCallbacks<'static>,
  used: bool,
}

#[napi]
impl RemoteCallbacks {
  #[napi(constructor)]
  #[allow(clippy::new_without_default)]
  pub fn new() -> RemoteCallbacks {
    RemoteCallbacks {
      inner: git2::RemoteCallbacks::new(),
      used: false,
    }
  }

  #[napi]
  /// The callback through which to fetch credentials if required.
  ///
  /// # Example
  ///
  /// Prepare a callback to authenticate using the `$HOME/.ssh/id_rsa` SSH key, and
  /// extracting the username from the URL (i.e. git@github.com:rust-lang/git2-rs.git):
  ///
  /// ```js
  /// import { join } from 'node:path'
  /// import { homedir } from 'node:os'
  ///
  /// import { Cred, FetchOptions, RemoteCallbacks, RepoBuilder, credTypeContains } from '@napi-rs/simple-git'
  ///
  /// const builder = new RepoBuilder()
  ///
  /// const remoteCallbacks = new RemoteCallbacks()
  /// .credentials((cred) => {
  ///   return Cred.sshKey(cred.username, null, join(homedir(), '.ssh', 'id_rsa'), null)
  /// })
  ///
  /// const fetchOptions = new FetchOptions().depth(0).remoteCallback(remoteCallbacks)
  ///
  /// const repo = builder.branch('master')
  ///  .fetchOptions(fetchOptions)
  ///  .clone("git@github.com:rust-lang/git2-rs.git", "git2-rs")
  /// ```
  pub fn credentials(
    &mut self,
    env: Env,
    callback: Function<CredInfo, &'static mut Cred>,
  ) -> Result<&Self> {
    let func_ref = callback.create_ref()?;
    self
      .inner
      .credentials(move |url: &str, username_from_url, cred| {
        func_ref
          .borrow_back(&env)
          .and_then(|cb| {
            cb.call(CredInfo {
              cred_type: cred.bits(),
              url: url.to_string(),
              username: username_from_url.unwrap_or("git").to_string(),
            })
          })
          .map_err(|err| {
            git2::Error::new(
              ErrorCode::Auth,
              ErrorClass::Callback,
              format!("Call credentials callback failed {err}"),
            )
          })
          .and_then(|cred| {
            if cred.used {
              return Err(git2::Error::new(
                ErrorCode::Auth,
                ErrorClass::Callback,
                "Cred can only be used once",
              ));
            }
            let mut c = git2::Cred::default()?;
            mem::swap(&mut c, &mut cred.inner);
            cred.used = true;
            Ok(c)
          })
      });
    Ok(self)
  }

  #[napi]
  /// The callback through which progress is monitored.
  pub fn transfer_progress(&mut self, env: Env, callback: FunctionRef<Progress, ()>) -> &Self {
    self.inner.transfer_progress(move |p| {
      callback
        .borrow_back(&env)
        .and_then(|cb| cb.call(p.into()))
        .is_ok()
    });
    self
  }

  #[napi]
  /// The callback through which progress of push transfer is monitored.
  ///
  /// The callback receives a single `PushTransferProgress` object describing how
  /// many objects have been processed and how many bytes have been sent.
  pub fn push_transfer_progress(
    &mut self,
    env: Env,
    callback: FunctionRef<PushTransferProgress, ()>,
  ) -> &Self {
    self
      .inner
      .push_transfer_progress(move |current, total, bytes| {
        if let Err(err) = callback.borrow_back(&env).and_then(|cb| {
          cb.call(PushTransferProgress {
            current: current as u32,
            total: total as u32,
            bytes: bytes as u32,
          })
        }) {
          eprintln!("Push transfer progress callback failed: {err}");
        }
      });
    self
  }

  #[napi]
  /// Set a callback to get invoked for each updated reference on a push.
  ///
  /// The callback is invoked once per reference with a single
  /// `PushUpdateReference` object. `status` is `null` when the reference was
  /// updated successfully; otherwise it is the server's rejection reason.
  pub fn push_update_reference(
    &mut self,
    env: Env,
    callback: FunctionRef<PushUpdateReference, ()>,
  ) -> &Self {
    self.inner.push_update_reference(move |refname, status| {
      callback
        .borrow_back(&env)
        .and_then(|cb| {
          cb.call(PushUpdateReference {
            refname: refname.to_string(),
            status: match status {
              Some(s) => Either::A(s.to_string()),
              None => Either::B(Null),
            },
          })
        })
        .map_err(|err| {
          git2::Error::new(
            ErrorCode::GenericError,
            ErrorClass::Callback,
            format!("Call push_update_reference callback failed {err}"),
          )
        })
    });
    self
  }
}

#[napi]
pub struct FetchOptions {
  pub(crate) inner: git2::FetchOptions<'static>,
  pub(crate) used: bool,
  /// `true` once `remote_callback` has attached JS-backed `RemoteCallbacks`.
  /// `fetch_async` rejects options with callbacks because they cannot be run
  /// off the main JS thread.
  pub(crate) has_remote_callbacks: bool,
}

#[napi]
impl FetchOptions {
  #[napi(constructor)]
  #[allow(clippy::new_without_default)]
  pub fn new() -> FetchOptions {
    FetchOptions {
      inner: git2::FetchOptions::new(),
      used: false,
      has_remote_callbacks: false,
    }
  }

  #[napi]
  /// Set the callbacks to use for the fetch operation.
  pub fn remote_callback(&mut self, callback: &mut RemoteCallbacks) -> Result<&Self> {
    if callback.used {
      return Err(Error::new(
        Status::GenericFailure,
        "RemoteCallbacks can only be used once".to_string(),
      ));
    }
    let mut cbs = git2::RemoteCallbacks::default();
    mem::swap(&mut cbs, &mut callback.inner);
    self.inner.remote_callbacks(cbs);
    callback.used = true;
    self.has_remote_callbacks = true;
    Ok(self)
  }

  #[napi]
  /// Set the proxy options to use for the fetch operation.
  pub fn proxy_options(&mut self, options: &mut ProxyOptions) -> Result<&Self> {
    if options.used {
      return Err(Error::new(
        Status::GenericFailure,
        "ProxyOptions can only be used once".to_string(),
      ));
    }
    let mut opts = git2::ProxyOptions::default();
    mem::swap(&mut opts, &mut options.inner);
    self.inner.proxy_options(opts);
    options.used = true;
    Ok(self)
  }

  #[napi]
  /// Set whether to perform a prune after the fetch.
  pub fn prune(&mut self, prune: FetchPrune) -> &Self {
    self.inner.prune(prune.into());
    self
  }

  #[napi]
  /// Set whether to write the results to FETCH_HEAD.
  ///
  /// Defaults to `true`.
  pub fn update_fetchhead(&mut self, update: bool) -> &Self {
    self.inner.update_fetchhead(update);
    self
  }

  #[napi]
  /// Set fetch depth, a value less or equal to 0 is interpreted as pull
  /// everything (effectively the same as not declaring a limit depth).
  ///
  // FIXME(blyxyas): We currently don't have a test for shallow functions
  // because libgit2 doesn't support local shallow clones.
  // https://github.com/rust-lang/git2-rs/pull/979#issuecomment-1716299900
  pub fn depth(&mut self, depth: i32) -> &Self {
    self.inner.depth(depth);
    self
  }

  #[napi]
  /// Set how to behave regarding tags on the remote, such as auto-downloading
  /// tags for objects we're downloading or downloading all of them.
  ///
  /// The default is to auto-follow tags.
  pub fn download_tags(&mut self, opt: AutotagOption) -> &Self {
    self.inner.download_tags(opt.into());
    self
  }

  #[napi]
  /// Set remote redirection settings; whether redirects to another host are
  /// permitted.
  ///
  /// By default, git will follow a redirect on the initial request
  /// (`/info/refs`), but not subsequent requests.
  pub fn follow_redirects(&mut self, opt: RemoteRedirect) -> &Self {
    self.inner.follow_redirects(opt.into());
    self
  }

  #[napi]
  /// Set extra headers for this fetch operation.
  ///
  /// Throws if any header contains an interior NUL byte.
  pub fn custom_headers(&mut self, headers: Vec<String>) -> Result<&Self> {
    reject_interior_nul(&headers, "custom header")?;
    self
      .inner
      .custom_headers(&headers.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    Ok(self)
  }
}

#[napi]
pub struct PushOptions {
  pub(crate) inner: git2::PushOptions<'static>,
  pub(crate) used: bool,
  /// `true` once `remote_callback` has attached JS-backed `RemoteCallbacks`.
  /// `push_async` rejects options with callbacks because they cannot be run
  /// off the main JS thread.
  pub(crate) has_remote_callbacks: bool,
}

#[napi]
impl PushOptions {
  #[napi(constructor)]
  #[allow(clippy::new_without_default)]
  pub fn new() -> PushOptions {
    PushOptions {
      inner: git2::PushOptions::new(),
      used: false,
      has_remote_callbacks: false,
    }
  }

  #[napi]
  /// Set the callbacks to use for the push operation.
  pub fn remote_callback(&mut self, callback: &mut RemoteCallbacks) -> Result<&Self> {
    if callback.used {
      return Err(Error::new(
        Status::GenericFailure,
        "RemoteCallbacks can only be used once".to_string(),
      ));
    }
    let mut cbs = git2::RemoteCallbacks::default();
    mem::swap(&mut cbs, &mut callback.inner);
    self.inner.remote_callbacks(cbs);
    callback.used = true;
    self.has_remote_callbacks = true;
    Ok(self)
  }

  #[napi]
  /// Set the proxy options to use for the push operation.
  pub fn proxy_options(&mut self, options: &mut ProxyOptions) -> Result<&Self> {
    if options.used {
      return Err(Error::new(
        Status::GenericFailure,
        "ProxyOptions can only be used once".to_string(),
      ));
    }
    let mut opts = git2::ProxyOptions::default();
    mem::swap(&mut opts, &mut options.inner);
    self.inner.proxy_options(opts);
    options.used = true;
    Ok(self)
  }

  #[napi]
  /// If the transport being used to push to the remote requires the creation
  /// of a pack file, this controls the number of worker threads used by the
  /// packbuilder when creating that pack file to be sent to the remote.
  ///
  /// If set to 0 the packbuilder will auto-detect the number of threads to
  /// create, and the default value is 1.
  pub fn packbuilder_parallelism(&mut self, parallel: u32) -> &Self {
    self.inner.packbuilder_parallelism(parallel);
    self
  }

  #[napi]
  /// Set remote redirection settings; whether redirects to another host are
  /// permitted.
  ///
  /// By default, git will follow a redirect on the initial request
  /// (`/info/refs`), but not subsequent requests.
  pub fn follow_redirects(&mut self, opt: RemoteRedirect) -> &Self {
    self.inner.follow_redirects(opt.into());
    self
  }

  #[napi]
  /// Set extra headers for this push operation.
  ///
  /// Throws if any header contains an interior NUL byte.
  pub fn custom_headers(&mut self, headers: Vec<String>) -> Result<&Self> {
    reject_interior_nul(&headers, "custom header")?;
    self
      .inner
      .custom_headers(&headers.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    Ok(self)
  }

  #[napi]
  /// Set "push options" to deliver to the remote.
  ///
  /// Throws if any push option contains an interior NUL byte.
  pub fn remote_push_options(&mut self, options: Vec<String>) -> Result<&Self> {
    reject_interior_nul(&options, "remote push option")?;
    self
      .inner
      .remote_push_options(&options.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    Ok(self)
  }
}

/// git2 builds a `CString` from each of these strings with an internal
/// `unwrap`, so an interior NUL byte would panic — and because this crate does
/// not opt into `catch_unwind`, that panic aborts the whole Node process.
/// Reject NULs up front so a bad value surfaces as an ordinary JS error.
fn reject_interior_nul(values: &[String], what: &str) -> Result<()> {
  for value in values {
    if value.contains('\0') {
      return Err(Error::new(
        Status::InvalidArg,
        format!("{what} contains an interior NUL byte"),
      ));
    }
  }
  Ok(())
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

impl<'a> From<git2::Progress<'a>> for Progress {
  fn from(progress: git2::Progress) -> Self {
    Progress {
      total_objects: progress.total_objects() as u32,
      indexed_objects: progress.indexed_objects() as u32,
      received_objects: progress.received_objects() as u32,
      local_objects: progress.local_objects() as u32,
      total_deltas: progress.total_deltas() as u32,
      indexed_deltas: progress.indexed_deltas() as u32,
      received_bytes: progress.received_bytes() as u32,
    }
  }
}

#[napi(object)]
pub struct PushTransferProgress {
  pub current: u32,
  pub total: u32,
  pub bytes: u32,
}

#[napi(object)]
/// A single reference update reported during a push.
pub struct PushUpdateReference {
  /// The full name of the reference that was updated (e.g.
  /// `refs/heads/main`).
  pub refname: String,
  /// `null` when the reference was updated successfully; otherwise the
  /// server's rejection reason.
  pub status: Either<String, Null>,
}

#[napi]
pub struct ProxyOptions {
  inner: git2::ProxyOptions<'static>,
  used: bool,
}

#[napi]
impl ProxyOptions {
  #[napi(constructor)]
  #[allow(clippy::new_without_default)]
  pub fn new() -> ProxyOptions {
    ProxyOptions {
      inner: git2::ProxyOptions::new(),
      used: false,
    }
  }

  #[napi]
  /// Try to auto-detect the proxy from the git configuration.
  ///
  /// Note that this will override `url` specified before.
  pub fn auto(&mut self) -> &Self {
    self.inner.auto();
    self
  }

  #[napi]
  /// Specify the exact URL of the proxy to use.
  ///
  /// Note that this will override `auto` specified before.
  pub fn url(&mut self, url: String) -> &Self {
    self.inner.url(url.as_str());
    self
  }
}

#[napi]
pub struct Cred {
  pub(crate) inner: git2::Cred,
  used: bool,
}

#[napi]
impl Cred {
  #[napi(constructor)]
  #[allow(clippy::new_without_default)]
  /// Create a "default" credential usable for Negotiate mechanisms like NTLM
  /// or Kerberos authentication.
  pub fn new() -> Result<Self> {
    Ok(Self {
      inner: git2::Cred::default().convert("Create Cred failed")?,
      used: false,
    })
  }

  #[napi(factory)]
  /// Create a new ssh key credential object used for querying an ssh-agent.
  ///
  /// The username specified is the username to authenticate.
  pub fn ssh_key_from_agent(username: String) -> Result<Self> {
    Ok(Self {
      inner: git2::Cred::ssh_key_from_agent(username.as_str()).convert("Create Cred failed")?,
      used: false,
    })
  }

  #[napi(factory)]
  /// Create a new passphrase-protected ssh key credential object.
  pub fn ssh_key(
    username: String,
    publickey: Option<String>,
    privatekey: String,
    passphrase: Option<String>,
  ) -> Result<Self> {
    Ok(Self {
      inner: git2::Cred::ssh_key(
        username.as_str(),
        publickey.as_ref().map(Path::new),
        std::path::Path::new(&privatekey),
        passphrase.as_deref(),
      )
      .convert("Create Cred failed")?,
      used: false,
    })
  }

  #[napi(factory)]
  /// Create a new ssh key credential object reading the keys from memory.
  pub fn ssh_key_from_memory(
    username: String,
    publickey: Option<String>,
    privatekey: String,
    passphrase: Option<String>,
  ) -> Result<Self> {
    Ok(Self {
      inner: git2::Cred::ssh_key_from_memory(
        username.as_str(),
        publickey.as_deref(),
        privatekey.as_str(),
        passphrase.as_deref(),
      )
      .convert("Create Cred failed")?,
      used: false,
    })
  }

  #[napi(factory)]
  /// Create a new plain-text username and password credential object.
  pub fn userpass_plaintext(username: String, password: String) -> Result<Self> {
    Ok(Self {
      inner: git2::Cred::userpass_plaintext(username.as_str(), password.as_str())
        .convert("Create Cred failed")?,
      used: false,
    })
  }

  #[napi(factory)]
  /// Create a credential to specify a username.
  ///
  /// This is used with ssh authentication to query for the username if none is
  /// specified in the URL.
  pub fn username(username: String) -> Result<Self> {
    Ok(Self {
      inner: git2::Cred::username(username.as_str()).convert("Create Cred failed")?,
      used: false,
    })
  }

  #[napi]
  /// Check whether a credential object contains username information.
  pub fn has_username(&self) -> bool {
    self.inner.has_username()
  }

  #[napi]
  #[allow(clippy::unnecessary_cast)] // git_credtype_t is i32 on MSVC, u32 elsewhere
  /// Return the type of credentials that this object represents.
  ///
  /// The value is the raw `CredentialType` bitset (an OR-able `number`); test
  /// individual bits with `credTypeContains` and the `CredentialType` constants.
  pub fn credtype(&self) -> u32 {
    // Normalize the platform-variant raw `git_credtype_t` to a portable u32. The
    // values match `git2::CredentialType` (always u32), so callers can mask with
    // CredentialType.* / credTypeContains.
    self.inner.credtype() as u32
  }
}

#[napi]
/// Check whether a raw credential-type bitset contains a given `CredentialType`
/// bit.
///
/// `cred_type` is the raw value (e.g. `CredInfo.credType` or `Cred.credtype()`);
/// `another` is one of the `CredentialType` constants. Returns
/// `(cred_type & another) === another`.
pub fn cred_type_contains(cred_type: u32, another: CredentialType) -> bool {
  let another_bits = Into::<git2::CredentialType>::into(another).bits();
  (cred_type & another_bits) == another_bits
}
