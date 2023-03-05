use std::io::Error;
use std::str::FromStr;
use tokio_serial::{Parity, SerialPort, SerialPortBuilderExt, SerialStream, StopBits};

pub struct PortSettings {
    name: String,
    speed: u32,
    parity: Parity,
    stop_bits: StopBits,
}

impl FromStr for PortSettings {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name: String = s.chars().take_while(|c| *c != ':').collect(); //&s[..delim_pos];
        let params: String = s.chars().skip_while(|c| *c != ':').skip(1).collect();
        let info: Vec<&str> = params.split('-').collect();

        if name.len() < 4 {
            return Err("name is too short");
        }

        if info.len() < 4 {
            return Err("not enough port parameters");
        }

        let speed = u32::from_str(info[0]).map_err(|_| "invalid speed")?;
        let parity = match info[2] {
            "N" => Ok(Parity::None),
            "E" => Ok(Parity::Even),
            "O" => Ok(Parity::Odd),
            _ => Err("invalid parity"),
        }?;

        let stop_bits = match info[3] {
            "1" => Ok(StopBits::One),
            "2" => Ok(StopBits::Two),
            _ => Err("invalid stop bits"),
        }?;

        Ok(PortSettings {
            name,
            speed,
            parity,
            stop_bits,
        })
    }
}

pub fn build(parameters: PortSettings) -> Result<SerialStream, Error> {
    let port = tokio_serial::new(parameters.name, parameters.speed)
        .parity(parameters.parity)
        .stop_bits(parameters.stop_bits)
        .open_native_async()?;

    port.clear(tokio_serial::ClearBuffer::All)?;
    Ok(port)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_settings() {
        assert_eq!(PortSettings::from_str(":").is_err(), true);
        assert_eq!(PortSettings::from_str("").is_err(), true);
        assert_eq!(PortSettings::from_str("/dev/ttyUSB0").is_err(), true);
        assert_eq!(PortSettings::from_str("/dev/ttyUSB0:").is_err(), true);
        assert_eq!(PortSettings::from_str("/dev/ttyUSB0:9600").is_err(), true);
        assert_eq!(PortSettings::from_str("/dev/ttyUSB0:9600-8").is_err(), true);
        assert_eq!(
            PortSettings::from_str("/dev/ttyUSB0:9600-8-N").is_err(),
            true
        );
        let correct = PortSettings::from_str("/dev/ttyUSB0:9600-8-N-1").unwrap();
        assert_eq!(correct.name, "/dev/ttyUSB0");
        assert_eq!(correct.speed, 9600);
        assert_eq!(correct.parity, Parity::None);
        assert_eq!(correct.stop_bits, StopBits::One);
    }
}
