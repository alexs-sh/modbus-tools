use log::{debug, trace};
pub fn log_data(name: &str, txt: &str, data: &[u8]) {
    if data.len() > 0 {
        trace!("{} {} {:?}", name, txt, data);
    }
}

pub fn log_message<A, B>(name: &A, data: &B)
where
    A: std::fmt::Display,
    B: std::fmt::Display,
{
    debug!("{} {}", name, data);
}
