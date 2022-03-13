#![deny(clippy::all)]

use std::path::PathBuf;

use dashmap::{mapref::one::RefMut, DashMap};
use napi::{
  bindgen_prelude::{AbortSignal, AsyncTask},
  Task,
};
use napi_derive::napi;
use once_cell::sync::Lazy;

#[derive(Default)]
struct GlobalLazyRepoCache(Lazy<DashMap<String, git2::Repository>>);

unsafe impl Sync for GlobalLazyRepoCache {}

static REPO_CACHE: GlobalLazyRepoCache = GlobalLazyRepoCache(Lazy::new(|| Default::default()));

#[inline]
fn create_or_insert(git_dir: &String) -> RefMut<'static, String, git2::Repository> {
  REPO_CACHE.0.entry(git_dir.clone()).or_insert_with(|| {
    git2::Repository::open(git_dir)
      .map_err(|err| panic!("Failed to open git repo: [{}], reason: {}", git_dir, err))
      .unwrap()
  })
}

pub struct GitDateTask {
  git_dir: String,
  filepath: String,
}

#[napi]
impl Task for GitDateTask {
  type Output = i64;
  type JsValue = i64;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let repo = create_or_insert(&self.git_dir);
    get_file_modified_date(repo, &self.filepath)
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
pub fn get_file_latest_modified_date_by_git_async(
  git_dir: String,
  filepath: String,
  signal: Option<AbortSignal>,
) -> AsyncTask<GitDateTask> {
  AsyncTask::with_optional_signal(GitDateTask { git_dir, filepath }, signal)
}

#[napi]
pub fn get_file_latest_modified_date_by_git(
  git_dir: String,
  filepath: String,
) -> Result<i64, napi::Error> {
  let repo = create_or_insert(&git_dir);
  get_file_modified_date(repo, &filepath)
}

#[inline]
fn get_file_modified_date(
  repo: RefMut<String, git2::Repository>,
  filepath: &str,
) -> napi::Result<i64> {
  repo
    .revwalk()
    .map_err(|err| {
      napi::Error::new(
        napi::Status::GenericFailure,
        format!("Failed to create revwalk: {}", err),
      )
    })
    .and_then(|mut rev_walk| {
      let head = repo.head().map_err(|err| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("Failed to get git head: {}", err),
        )
      })?;
      rev_walk
        .push(head.target().ok_or_else(|| {
          napi::Error::new(
            napi::Status::InvalidArg,
            format!("Git repo head point to non-object"),
          )
        })?)
        .map_err(|err| {
          napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to push git head to revwalk: {}", err),
          )
        })?;
      rev_walk
        .set_sorting(git2::Sort::TIME & git2::Sort::TOPOLOGICAL)
        .map_err(|err| {
          napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to set rev walk sort mode: {}", err),
          )
        })?;
      rev_walk
        .find_map(|oid| {
          let oid = oid.ok()?;
          let commit = repo.find_commit(oid).ok()?;
          let tree = commit.tree().ok()?;
          let mut this_commit = None;
          if tree.get_path(&PathBuf::from(filepath)).is_ok() {
            this_commit = Some(commit.time().seconds() * 1000);
          }
          this_commit
        })
        .ok_or_else(|| {
          napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to find commit for [{}]", filepath),
          )
        })
    })
}
