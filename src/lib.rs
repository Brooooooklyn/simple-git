#![deny(clippy::all)]

pub mod blob;
pub mod commit;
pub mod config;
pub mod deltas;
pub mod diff;
mod error;
pub mod file_modification;
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
