use std::str::FromStr;

#[derive(Clone)]
pub enum TransportAddress {
    Tcp(String),
    Udp(String),
    Serial(String),
}

impl TransportAddress {
    pub fn get(&self) -> &str {
        match self {
            TransportAddress::Tcp(address) => address,
            TransportAddress::Udp(address) => address,
            TransportAddress::Serial(address) => address,
        }
    }
}

#[derive(Clone)]
pub struct Settings {
    pub address: TransportAddress,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            address: TransportAddress::Tcp("0.0.0.0:502".to_owned()),
        }
    }
}

impl FromStr for TransportAddress {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split(':').next().map_or(Err(()), |tp| {
            let begin = tp.len() + 1;
            if begin >= s.len() {
                return Err(());
            }

            let remain = &s[begin..];
            match tp {
                "tcp" => Ok(TransportAddress::Tcp(remain.to_owned())),
                "udp" => Ok(TransportAddress::Udp(remain.to_owned())),
                "serial" => Ok(TransportAddress::Serial(remain.to_owned())),
                _ => Err(()),
            }
        })
    }
}

#[cfg(test)]

mod test {

    use super::*;

    #[test]
    fn transport_address() {
        let address = TransportAddress::from_str("");
        assert!(address.is_err());

        let address = TransportAddress::from_str("unknown:/dev/tty0");
        assert!(address.is_err());

        let address = TransportAddress::from_str("tcp:127.0.0.1:502").unwrap();
        match address {
            TransportAddress::Tcp(ip) => {
                assert_eq!(ip, "127.0.0.1:502");
            }
            _ => unreachable!(),
        };

        let address = TransportAddress::from_str("udp:127.0.0.1:502").unwrap();
        match address {
            TransportAddress::Udp(ip) => {
                assert_eq!(ip, "127.0.0.1:502");
            }
            _ => unreachable!(),
        };

        let address = TransportAddress::from_str("serial:/dev/tty0").unwrap();
        match address {
            TransportAddress::Serial(name) => {
                assert_eq!(name, "/dev/tty0");
            }
            _ => unreachable!(),
        };
    }
}
