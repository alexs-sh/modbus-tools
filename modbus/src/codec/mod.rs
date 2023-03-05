pub mod context;
pub mod error;
pub mod mbap;
pub mod pduext;
pub mod rtuext;
pub mod slave;

#[macro_export]
macro_rules! wait {
    ($op:expr) => {
        if let Some(x) = $op {
            x
        } else {
            return Ok(None);
        }
    };
}

pub(crate) use wait;
