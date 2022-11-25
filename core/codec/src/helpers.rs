use log::{debug, trace};

pub fn log_data(name: &str, txt: &str, data: &[u8]) {
    if data.len() > 0 {
        trace!("{} {}: {:?}", name, txt, data);
    }
}

pub fn log_frame<A, B, C>(name: &A, txt: &B, data: &C)
where
    A: std::fmt::Display,
    B: std::fmt::Display,
    C: std::fmt::Debug,
{
    debug!("{} {}: {:?}", name, txt, data);
}
