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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_code() {
        assert_eq!(u8::from(Code::IllegalFunction), 0x01);
        assert_eq!(u8::from(Code::IllegalDataAddress), 0x02);
        assert_eq!(u8::from(Code::IllegalDataValue), 0x03);
        assert_eq!(u8::from(Code::SlaveDeviceFailure), 0x04);
        assert_eq!(u8::from(Code::Acknowledge), 0x05);
        assert_eq!(u8::from(Code::SlaveDeviceBusy), 0x06);
        assert_eq!(u8::from(Code::MemoryParityError), 0x08);
        assert_eq!(u8::from(Code::GatewayPathUnavailable), 0x0A);
        assert_eq!(u8::from(Code::GatewayTargetDeciveFailedToRespond), 0x0B);
    }
}
