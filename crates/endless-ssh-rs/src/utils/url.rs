use color_eyre::eyre;
use url::Url;

/// Adds a segment to a Url
/// # Errors
/// When the Url given is relative
#[cfg_attr(not(test), expect(unused, reason = "Library code"))]
pub fn add_segments(mut base_url: Url, segments: &[&str]) -> Result<Url, eyre::Report> {
    {
        let mut s = base_url
            .path_segments_mut()
            .map_err(|()| eyre::Report::msg("Url is not a base"))?;
        s.extend(segments);
    }

    Ok(base_url)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use url::Url;

    use crate::utils::url::add_segments;

    #[test]
    fn add_single_segment() {
        let url = Url::from_str("https://example.com").unwrap();

        let new_url = add_segments(url, &["foobar"]);

        assert!(matches!(
            new_url.map(Into::<String>::into).as_deref(),
            Ok("https://example.com/foobar")
        ));
    }

    #[test]
    fn multiple_segments() {
        let url = Url::from_str("https://example.com").unwrap();

        let new_url = add_segments(url, &["foo", "bar"]);

        assert!(matches!(
            new_url.map(Into::<String>::into).as_deref(),
            Ok("https://example.com/foo/bar")
        ));
    }
}
