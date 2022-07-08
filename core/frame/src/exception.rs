use std::convert::From;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Code {
    IllegalFunction = 0x01,
    IllegalDataAddress = 0x02,
    IllegalDataValue = 0x03,
    SlaveDeviceFailure = 0x04,
    Acknowledge = 0x05,
    SlaveDeviceBusy = 0x06,
    MemoryParityError = 0x08,
    GatewayPathUnavailable = 0x0A,
    GatewayTargetDeciveFailedToRespond = 0x0B,
}

impl From<Code> for u8 {
    fn from(value: Code) -> u8 {
        value as u8
    }
}
