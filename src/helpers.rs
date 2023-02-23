#[macro_export]
macro_rules! wrap_and_report {
    ($level:expr, $error:expr, $message:expr) => {
        {
            let wrapped = Into::<color_eyre::Report>::into($error).wrap_err($message);

            tracing::event!($level, error = %wrapped, error = ?wrapped.source().unwrap());

            wrapped
        }
    };
}
