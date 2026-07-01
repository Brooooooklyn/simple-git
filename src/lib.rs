#![deny(clippy::all)]

pub(crate) use error::codes::{
  CodeInto, GitCode, Result, coded_error, disposed_error, ensure_alive,
};

pub mod blame;
pub mod blob;
pub mod branch;
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
pub(crate) mod util;
