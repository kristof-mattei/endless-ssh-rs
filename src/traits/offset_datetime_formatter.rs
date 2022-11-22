use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[allow(unused)]
pub(crate) fn offset_datetime_formatter(
    offset_datetime: &OffsetDateTime,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    match offset_datetime.format(&Rfc3339) {
        Ok(formatted) => f.write_str(&formatted),
        Err(e) => {
            write!(f, "Couldn't convert time to Rfc3339, error: {e:?}")
        },
    }
}
