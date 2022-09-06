use std::fmt::{Debug, Display};

pub(crate) fn pretty_format<T, E>(
    t: &Result<T, E>,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result
where
    T: Display,
    E: Debug,
{
    match t {
        Ok(t) => write!(f, "{}", t),
        Err(e) => write!(f, "{:?}", e),
    }
}
