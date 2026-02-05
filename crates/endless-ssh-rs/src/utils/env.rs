use color_eyre::eyre::{self, Context as _};
use url::Url;

/// Gets an environment variable and tries to convert it to an Url
///
/// # Errors
/// When the env value for `key` could not be converted to an Url
#[expect(unused, reason = "Library code")]
pub fn get_env_as_url(key: &str) -> Result<Url, eyre::Report> {
    let value = std::env::var(key)?;

    Url::parse(&value).wrap_err_with(|| format!("Couldn't convert {:?} to URL", value))
}
