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
      .map_err(|err| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("libgit2 error: {}", err),
        )
      })
      .and_then(|value| {
        value.ok_or_else(|| {
          napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to get commit for [{}]", &self.filepath),
          )
        })
      })
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
    .map_err(|err| {
      napi::Error::new(
        napi::Status::GenericFailure,
        format!("libgit2 error: {}", err),
      )
    })
    .and_then(|value| {
      value.ok_or_else(|| {
        napi::Error::new(
          napi::Status::GenericFailure,
          format!("Failed to get commit for [{}]", filepath),
        )
      })
    })
}

fn get_file_modified_date(
  repo: RefMut<String, git2::Repository>,
  filepath: &str,
) -> Result<Option<i64>, git2::Error> {
  let mut diff_options = git2::DiffOptions::new();
  diff_options.disable_pathspec_match(false);
  diff_options.pathspec(filepath);
  let mut rev_walk = repo.revwalk()?;
  rev_walk.push_head()?;
  rev_walk.set_sorting(git2::Sort::TIME & git2::Sort::TOPOLOGICAL)?;
  let path = PathBuf::from(filepath);
  Ok(
    rev_walk
      .by_ref()
      .filter_map(|oid| oid.ok())
      .find_map(|oid| {
        let commit = repo.find_commit(oid).ok()?;
        match commit.parent_count() {
          // commit with parent
          1 => {
            let tree = commit.tree().ok()?;
            if let Ok(parent) = commit.parent(0) {
              let parent_tree = parent.tree().ok()?;
              if let Ok(diff) =
                repo.diff_tree_to_tree(Some(&tree), Some(&parent_tree), Some(&mut diff_options))
              {
                if diff.deltas().len() > 0 {
                  return Some(commit.time().seconds() * 1000);
                }
              }
            }
          }
          // root commit
          0 => {
            let tree = commit.tree().ok()?;
            if tree.get_path(&path).is_ok() {
              return Some(commit.time().seconds() * 1000);
            }
          }
          // ignore merge commits
          _ => {}
        };
        None
      }),
  )
}
