use std::time::SystemTime;
use std::time::UNIX_EPOCH;

pub(crate) fn milliseconds_since_epoch() -> u128 {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Current time is before EPOCH? You're in trouble!");

    d.as_millis()
}
