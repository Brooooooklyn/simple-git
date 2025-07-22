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

impl From<libgit2_sys::git_credtype_t> for CredentialType {
  fn from(value: libgit2_sys::git_credtype_t) -> Self {
    match value {
      libgit2_sys::GIT_CREDTYPE_USERPASS_PLAINTEXT => CredentialType::UserPassPlaintext,
      libgit2_sys::GIT_CREDTYPE_SSH_KEY => CredentialType::SshKey,
      libgit2_sys::GIT_CREDTYPE_SSH_MEMORY => CredentialType::SshMemory,
      libgit2_sys::GIT_CREDTYPE_SSH_CUSTOM => CredentialType::SshCustom,
      libgit2_sys::GIT_CREDTYPE_DEFAULT => CredentialType::Default,
      libgit2_sys::GIT_CREDTYPE_SSH_INTERACTIVE => CredentialType::SshInteractive,
      libgit2_sys::GIT_CREDTYPE_USERNAME => CredentialType::Username,
      _ => CredentialType::Default,
    }
  }
}

impl From<git2::CredentialType> for CredentialType {
  fn from(value: git2::CredentialType) -> Self {
    match value {
      git2::CredentialType::USER_PASS_PLAINTEXT => CredentialType::UserPassPlaintext,
      git2::CredentialType::SSH_KEY => CredentialType::SshKey,
      git2::CredentialType::SSH_MEMORY => CredentialType::SshMemory,
      git2::CredentialType::SSH_CUSTOM => CredentialType::SshCustom,
      git2::CredentialType::DEFAULT => CredentialType::Default,
      git2::CredentialType::SSH_INTERACTIVE => CredentialType::SshInteractive,
      git2::CredentialType::USERNAME => CredentialType::Username,
      _ => CredentialType::Default,
    }
  }
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
  pub cred_type: CredentialType,
  pub url: String,
  pub username: String,
}

#[napi]
#[repr(u32)]
pub enum RemoteUpdateFlags {
  UpdateFetchHead = 1,
  ReportUnchanged = 2,
}

impl From<RemoteUpdateFlags> for git2::RemoteUpdateFlags {
  fn from(value: RemoteUpdateFlags) -> Self {
    match value {
      RemoteUpdateFlags::UpdateFetchHead => git2::RemoteUpdateFlags::UPDATE_FETCHHEAD,
      RemoteUpdateFlags::ReportUnchanged => git2::RemoteUpdateFlags::REPORT_UNCHANGED,
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
    let mut options = fetch_options
      .map(|o| {
        std::mem::swap(&mut o.inner, &mut default_fetch_options);
        default_fetch_options
      })
      .unwrap_or_default();
    self
      .inner
      .fetch(refspecs.as_slice(), Some(&mut options), None)
      .convert_without_message()
  }

  #[napi]
  /// Update the tips to the new state
  pub fn update_tips(
    &mut self,
    update_fetchhead: RemoteUpdateFlags,
    download_tags: AutotagOption,
    mut callbacks: Option<&mut RemoteCallbacks>,
    msg: Option<String>,
  ) -> Result<()> {
    let callbacks = callbacks.as_mut().map(|o| &mut o.inner);
    self
      .inner
      .update_tips(
        callbacks,
        update_fetchhead.into(),
        download_tags.into(),
        msg.as_deref(),
      )
      .convert_without_message()
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
              cred_type: cred.into(),
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

  #[napi(ts_args_type = "callback: (current: number, total: number, bytes: number) => void")]
  /// The callback through which progress of push transfer is monitored
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
}

#[napi]
pub struct FetchOptions {
  pub(crate) inner: git2::FetchOptions<'static>,
  pub(crate) used: bool,
}

#[napi]
impl FetchOptions {
  #[napi(constructor)]
  #[allow(clippy::new_without_default)]
  pub fn new() -> FetchOptions {
    FetchOptions {
      inner: git2::FetchOptions::new(),
      used: false,
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
  pub fn custom_headers(&mut self, headers: Vec<String>) -> &Self {
    self
      .inner
      .custom_headers(&headers.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    self
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
  /// Return the type of credentials that this object represents.
  pub fn credtype(&self) -> CredentialType {
    self.inner.credtype().into()
  }
}

#[napi]
/// Check whether a cred_type contains another credential type.
pub fn cred_type_contains(cred_type: CredentialType, another: CredentialType) -> bool {
  Into::<git2::CredentialType>::into(cred_type).contains(another.into())
}
