use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

// use tracing::field::Field;
// use tracing::field::Visit;

// pub struct DurationVisitor<'d> {
//     duration: &'d mut Duration,
// }

// impl<'d> Visit for DurationVisitor<'d> {
//     fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
//         write!(self.duration, "{} = {}{:03}", field.name() , value)
//     }
// }

struct DurationPrinter<'d> {
    duration: &'d Duration,
}

impl<'d> std::fmt::Debug for DurationPrinter<'d> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Duration")
            .field("duration", &self.duration)
            .finish()
    }
}

pub(crate) fn format_duration(duration: &Duration) -> String {
    format!("{}.{:03}", duration.as_secs(), duration.subsec_millis())
}

pub(crate) fn duration_since_epoch() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Current time is before EPOCH? You're in trouble!")
}
