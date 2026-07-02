use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::error::IntoNapiError;
use crate::{GitErrorCode, Result, ensure_alive};

/// `Number.MAX_SAFE_INTEGER` — largest integer a JS `number` (f64) holds losslessly.
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

#[napi]
/// The priority level a configuration entry or file applies to. Higher levels
/// take precedence; `Local` (the repository's own `.git/config`) is where
/// `set_*`/`remove_entry` write by default.
pub enum ConfigLevel {
  /// System-wide on Windows, for compatibility with portable git
  ProgramData,
  /// System-wide configuration file, e.g. /etc/gitconfig
  System,
  /// XDG-compatible configuration file, e.g. ~/.config/git/config
  Xdg,
  /// User-specific configuration, e.g. ~/.gitconfig
  Global,
  /// Repository specific config, e.g. $PWD/.git/config
  Local,
  /// Worktree specific configuration file, e.g. $GIT_DIR/config.worktree
  Worktree,
  /// Application specific configuration file
  App,
  /// Highest level available
  Highest,
}

impl From<git2::ConfigLevel> for ConfigLevel {
  fn from(value: git2::ConfigLevel) -> Self {
    match value {
      git2::ConfigLevel::ProgramData => ConfigLevel::ProgramData,
      git2::ConfigLevel::System => ConfigLevel::System,
      git2::ConfigLevel::XDG => ConfigLevel::Xdg,
      git2::ConfigLevel::Global => ConfigLevel::Global,
      git2::ConfigLevel::Local => ConfigLevel::Local,
      git2::ConfigLevel::Worktree => ConfigLevel::Worktree,
      git2::ConfigLevel::App => ConfigLevel::App,
      git2::ConfigLevel::Highest => ConfigLevel::Highest,
    }
  }
}

#[napi(object)]
/// A single configuration entry: its fully-qualified name, value, and the
/// level (file) it was read from.
pub struct ConfigEntry {
  pub name: String,
  pub value: String,
  pub level: ConfigLevel,
}

#[napi]
/// A git configuration store.
///
/// Obtain one with `Repository.config()` (a prioritized view of system, global
/// and repository config) or `Config.openDefault()` (system/global/XDG only).
pub struct Config {
  pub(crate) inner: git2::Config,
  /// Liveness flag. A repo-derived config (`Repository.config()`) shares the
  /// owning `Repository`'s flag, so it dies with the repo. A standalone config
  /// (`Config.openDefault()`) carries a fresh, never-flipped flag — it borrows
  /// no repository, so it stays valid after any `dispose()`.
  pub(crate) alive: Arc<AtomicBool>,
}

#[napi]
impl Config {
  #[napi(factory)]
  /// Open the global, XDG and system configuration files into a single
  /// prioritized config object that can be used when accessing default config
  /// data outside a repository.
  pub fn open_default() -> Result<Config> {
    Ok(Config {
      inner: git2::Config::open_default().convert_without_message()?,
      alive: Arc::new(AtomicBool::new(true)),
    })
  }

  #[napi]
  /// Get the value of a string config variable as an owned string.
  ///
  /// All config files are searched in order of their level (highest priority
  /// first) and the first occurrence is returned. Errors if the value is not
  /// valid utf-8 or the key is missing.
  pub fn get_string(&self, name: String) -> Result<String> {
    ensure_alive(&self.alive)?;
    self.inner.get_string(&name).convert_without_message()
  }

  #[napi]
  /// Get the value of a boolean config variable.
  pub fn get_boolean(&self, name: String) -> Result<bool> {
    ensure_alive(&self.alive)?;
    self.inner.get_bool(&name).convert_without_message()
  }

  #[napi]
  /// Get the value of an integer config variable, as a JS `number`.
  ///
  /// Reads the value as a 64-bit integer. Errors with `InvalidArg` when it lies
  /// outside the JS safe-integer range (±(2^53 − 1)), where a `number` would lose
  /// precision — use `getBigInt` for those.
  pub fn get_number(&self, name: String) -> Result<f64> {
    ensure_alive(&self.alive)?;
    let value = self.inner.get_i64(&name).convert_without_message()?;
    if !(-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&value) {
      return Err(Error::new(
        GitErrorCode::InvalidArg,
        format!(
          "Config value for `{name}` ({value}) exceeds Number.MAX_SAFE_INTEGER; use getBigInt"
        ),
      ));
    }
    Ok(value as f64)
  }

  #[napi]
  /// Get the value of an i64 config variable, as a JS `bigint`.
  ///
  /// Returns a `bigint` rather than a `number` so values beyond
  /// `Number.MAX_SAFE_INTEGER` (2^53 - 1) survive without truncation.
  pub fn get_big_int(&self, name: String) -> Result<BigInt> {
    ensure_alive(&self.alive)?;
    let value = self.inner.get_i64(&name).convert_without_message()?;
    Ok(BigInt::from(value))
  }

  #[napi]
  /// Set the value of a string config variable in the config file with the
  /// highest level (usually the local one).
  pub fn set_string(&mut self, name: String, value: String) -> Result<()> {
    ensure_alive(&self.alive)?;
    self.inner.set_str(&name, &value).convert_without_message()
  }

  #[napi]
  /// Set the value of a boolean config variable in the config file with the
  /// highest level (usually the local one).
  pub fn set_boolean(&mut self, name: String, value: bool) -> Result<()> {
    ensure_alive(&self.alive)?;
    self.inner.set_bool(&name, value).convert_without_message()
  }

  #[napi]
  /// Set the value of an integer config variable in the config file with the
  /// highest level (usually the local one). Takes a JS `number`.
  ///
  /// Errors with `InvalidArg` when `value` is not an integer or lies outside the
  /// JS safe-integer range (±(2^53 − 1)) — use `setBigInt` for larger magnitudes
  /// rather than silently truncating.
  pub fn set_number(&mut self, name: String, value: f64) -> Result<()> {
    ensure_alive(&self.alive)?;
    if value.fract() != 0.0 || value.abs() > MAX_SAFE_INTEGER as f64 {
      return Err(Error::new(
        GitErrorCode::InvalidArg,
        format!(
          "Config value for `{name}` ({value}) must be an integer within the JS safe-integer range; use setBigInt"
        ),
      ));
    }
    self
      .inner
      .set_i64(&name, value as i64)
      .convert_without_message()
  }

  #[napi]
  /// Set the value of an i64 config variable in the config file with the
  /// highest level (usually the local one). Takes a JS `bigint`.
  ///
  /// Errors with `InvalidArg` if the `bigint` does not fit losslessly in an
  /// i64 rather than silently truncating it.
  pub fn set_big_int(&mut self, name: String, value: BigInt) -> Result<()> {
    ensure_alive(&self.alive)?;
    let (value, lossless) = value.get_i64();
    if !lossless {
      return Err(Error::new(
        GitErrorCode::InvalidArg,
        format!("BigInt value for `{name}` does not fit in a 64-bit signed integer"),
      ));
    }
    self.inner.set_i64(&name, value).convert_without_message()
  }

  #[napi]
  /// Delete a config variable from the config file with the highest level
  /// (usually the local one).
  pub fn remove_entry(&mut self, name: String) -> Result<()> {
    ensure_alive(&self.alive)?;
    self.inner.remove(&name).convert_without_message()
  }

  #[napi]
  /// Create a read-only point-in-time snapshot of this configuration.
  ///
  /// A snapshot gives a consistent view for looking up complex values. Note
  /// that `get_*` on a live (non-snapshot) config re-reads the underlying
  /// files on each call.
  pub fn snapshot(&mut self) -> Result<Config> {
    ensure_alive(&self.alive)?;
    Ok(Config {
      inner: self.inner.snapshot().convert_without_message()?,
      // A snapshot of a repo-derived config must die with the repo, so it
      // inherits the same liveness flag rather than minting a fresh one.
      alive: self.alive.clone(),
    })
  }

  #[napi]
  /// List configuration entries, optionally filtered by a glob pattern.
  ///
  /// Each borrowed entry is eagerly materialized into an owned `ConfigEntry`.
  /// Entries whose name or value is not valid utf-8 are skipped.
  pub fn entries(&self, glob: Option<String>) -> Result<Vec<ConfigEntry>> {
    ensure_alive(&self.alive)?;
    let mut entries = self
      .inner
      .entries(glob.as_deref())
      .convert_without_message()?;
    let mut result = Vec::new();
    while let Some(entry) = entries.next() {
      let entry = entry.convert_without_message()?;
      let Ok(name) = entry.name() else {
        continue;
      };
      // `value()` panics when no value is defined, so guard with `has_value()`;
      // a valueless key (shorthand for boolean `true`) surfaces as an empty
      // string. Non-utf-8 values are skipped.
      let value = if entry.has_value() {
        match entry.value() {
          Ok(value) => value.to_owned(),
          Err(_) => continue,
        }
      } else {
        String::new()
      };
      result.push(ConfigEntry {
        name: name.to_owned(),
        value,
        level: entry.level().into(),
      });
    }
    Ok(result)
  }
}
