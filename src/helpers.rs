#[macro_export]
macro_rules! wrap_and_report {
    ($level:expr, $error:expr, $message:expr) => {
        {
            let wrapped = anyhow::Error::new($error).context($message);

            event!($level, error = %wrapped, error = ?wrapped.source().unwrap());

            wrapped
        }
    };
}
