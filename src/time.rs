use std::time::SystemTime;
use std::time::UNIX_EPOCH;

pub(crate) fn epochms() -> u128 {
    let d = SystemTime::now().duration_since(UNIX_EPOCH);
    d.map(|d| d.as_millis()).expect("Getting time failed")
}
