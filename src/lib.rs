#![deny(clippy::all)]

pub(crate) use error::codes::{
  CodeInto, GitErrorCode, Result, coded_error, disposed_error, ensure_alive,
};

pub mod blame;
pub mod blob;
pub mod branch;
pub mod buffer_diff;
pub mod checkout;
pub mod commit;
pub mod config;
pub mod deltas;
pub mod diff;
mod error;
pub mod file_modification;
pub mod index;
pub mod object;
pub mod reference;
pub mod remote;
pub mod repo;
pub mod repo_builder;
pub mod rev_walk;
pub mod signature;
pub mod status;
pub mod tag;
pub mod tree;
pub mod tree_builder;
pub(crate) mod util;

/// Initialize libgit2 (openssl/ssh subsystems + `git_libgit2_init`) when the
/// native module loads. Idempotent — internally a `Once`.
#[napi_derive::module_init]
fn init_libgit2() {
  libgit2_sys::init();
}
