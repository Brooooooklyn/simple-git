use std::path::PathBuf;

use dashmap::{mapref::one::RefMut, DashMap};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use once_cell::sync::Lazy;

use crate::reference::Reference;

#[derive(Default)]
pub(crate) struct GlobalLazyRepoCache(pub(crate) Lazy<DashMap<String, git2::Repository>>);

unsafe impl Sync for GlobalLazyRepoCache {}

pub(crate) static REPO_CACHE: GlobalLazyRepoCache =
  GlobalLazyRepoCache(Lazy::new(Default::default));

static INIT_GIT_CONFIG: Lazy<Result<()>> = Lazy::new(|| {
  // Handle the `failed to stat '/root/.gitconfig'; class=Config (7)` Error
  #[cfg(all(
    target_os = "linux",
    target_env = "gnu",
    any(target_arch = "x86_64", target_arch = "aarch64")
  ))]
  {
    if git2::Config::find_global().is_err() {
      if let Some(mut git_config_dir) = dirs::home_dir() {
        git_config_dir.push(".gitconfig");
        std::fs::write(&git_config_dir, "").map_err(|err| {
          Error::new(
            Status::GenericFailure,
            format!("Initialize {:?} failed {}", git_config_dir, err),
          )
        })?;
      }
    }
  }
  Ok(())
});

#[inline]
fn create_or_insert(git_dir: &str) -> Result<RefMut<'static, String, git2::Repository>> {
  REPO_CACHE
    .0
    .entry(git_dir.to_owned())
    .or_try_insert_with(|| {
      git2::Repository::open(git_dir).map_err(|err| {
        Error::new(
          Status::GenericFailure,
          format!("Failed to open git repo: [{}], reason: {}", git_dir, err,),
        )
      })
    })
}

#[napi]
pub struct Repository {
  repo: String,
}

#[napi]
impl Repository {
  #[napi]
  pub fn init(p: String) -> Result<Repository> {
    let path = p.clone();
    let _ = REPO_CACHE
      .0
      .entry(path.clone())
      .or_try_insert_with(move || {
        git2::Repository::init(&path).map_err(|err| {
          Error::new(
            Status::GenericFailure,
            format!("Failed to open git repo: [{}], reason: {}", path, err,),
          )
        })
      })?;
    Ok(Self { repo: p })
  }

  #[napi(constructor)]
  pub fn new(git_dir: String) -> Result<Self> {
    INIT_GIT_CONFIG.as_ref().map_err(|err| err.clone())?;
    Ok(Self { repo: git_dir })
  }

  #[napi]
  /// Retrieve and resolve the reference pointed at by HEAD.
  pub fn head(&self) -> Result<Reference> {
    Ok(Reference {
      repo: self.repo.clone(),
    })
  }

  #[napi]
  pub fn get_file_latest_modified_date(&self, filepath: String) -> Result<i64> {
    let repo = create_or_insert(&self.repo)?;
    get_file_modified_date(&repo, &filepath)
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

  #[napi]
  pub fn get_file_latest_modified_date_async(
    &self,
    filepath: String,
    signal: Option<AbortSignal>,
  ) -> AsyncTask<GitDateTask> {
    AsyncTask::with_optional_signal(
      GitDateTask {
        repo: self.repo.clone(),
        filepath,
      },
      signal,
    )
  }
}

pub struct GitDateTask {
  repo: String,
  filepath: String,
}

#[napi]
impl Task for GitDateTask {
  type Output = i64;
  type JsValue = i64;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    let repo = create_or_insert(&self.repo)?;
    get_file_modified_date(&repo, &self.filepath)
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

fn get_file_modified_date(
  repo: &git2::Repository,
  filepath: &str,
) -> std::result::Result<Option<i64>, git2::Error> {
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
