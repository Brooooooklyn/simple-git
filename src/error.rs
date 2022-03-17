pub(crate) trait IntoNapiError: Sized {
  type Associate;

  fn convert<S: AsRef<str>>(self, msg: S) -> Result<Self::Associate, napi::Error>;

  fn convert_without_message(self) -> Result<Self::Associate, napi::Error>;
}

impl<T> IntoNapiError for Result<T, git2::Error> {
  type Associate = T;

  #[inline]
  fn convert<S: AsRef<str>>(self, msg: S) -> Result<T, napi::Error> {
    self.map_err(|err| {
      napi::Error::new(
        napi::Status::GenericFailure,
        format!("{}: {}", msg.as_ref(), err),
      )
    })
  }

  #[inline]
  fn convert_without_message(self) -> Result<Self::Associate, napi::Error> {
    self.map_err(|err| {
      napi::Error::new(
        napi::Status::GenericFailure,
        format!("libgit2 error: {}", err),
      )
    })
  }
}

pub trait NotNullError {
  type Associate;

  fn expect_not_null(self, msg: String) -> Result<Self::Associate, napi::Error>;
}

impl<T> NotNullError for Option<T> {
  type Associate = T;

  #[inline]
  fn expect_not_null(self, msg: String) -> Result<T, napi::Error> {
    self.ok_or_else(|| napi::Error::new(napi::Status::GenericFailure, msg))
  }
}
