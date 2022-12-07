extern crate frame;
extern crate transport;

use env_logger::Builder;
use frame::exception::Code;
use frame::{
    data::Data, RequestFrame, RequestPdu, ResponseFrame, ResponsePdu, MAX_NCOILS, MAX_NREGS,
};
use log::{info, LevelFilter};
use tokio::signal;

use std::env;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use transport::builder;
use transport::{settings::Settings, settings::TransportAddress, Response};

#[derive(PartialEq, Eq, Hash)]
struct Address {
    slave: u8,
    func: u8,
    address: u16,
}

struct Memory {
    values: std::collections::HashMap<Address, u16>,
}

impl Memory {
    fn read_coils(&self, slave: u8, func: u8, address: u16, output: &mut [bool]) -> usize {
        let count = output.len();
        for (i, v) in output.iter_mut().enumerate().take(count) {
            let address = Address {
                slave,
                func,
                address: address + i as u16,
            };

            *v = self
                .values
                .get(&address)
                .map(|value| *value != 0)
                .unwrap_or(false);
        }
        count
    }

    fn read_registers(&self, slave: u8, func: u8, address: u16, output: &mut [u16]) -> usize {
        let count = output.len();
        for (i, v) in output.iter_mut().enumerate().take(count) {
            let address = Address {
                slave,
                func,
                address: address + i as u16,
            };

            *v = *self.values.get(&address).unwrap_or(&0);
        }
        count
    }

    fn write_coils(&mut self, slave: u8, func: u8, address: u16, input: &[bool]) -> usize {
        let count = input.len();
        for (i, v) in input.iter().enumerate().take(count) {
            let address = Address {
                slave,
                func,
                address: address + i as u16,
            };
            self.values.insert(address, *v as u16);
        }
        count
    }

    fn write_registers(&mut self, slave: u8, func: u8, address: u16, input: &[u16]) -> usize {
        let count = input.len();
        for (i, v) in input.iter().enumerate().take(count) {
            let address = Address {
                slave,
                func,
                address: address + i as u16,
            };
            self.values.insert(address, *v);
        }
        count
    }

    pub fn process(&mut self, request: &RequestFrame) -> ResponseFrame {
        let slave = request.slave;
        let func = request.pdu.func().unwrap();
        let mut coils = [false; MAX_NCOILS];
        let mut regs = [0u16; MAX_NREGS];
        let pdu = match &request.pdu {
            RequestPdu::ReadCoils { nobjs, address } => {
                let res = self.read_coils(slave, func, *address, &mut coils[..*nobjs as usize]);
                ResponsePdu::ReadCoils {
                    nobjs: *nobjs,
                    data: Data::coils(&coils[..res]),
                }
            }
            RequestPdu::ReadDiscreteInputs { nobjs, address } => {
                let res = self.read_coils(slave, func, *address, &mut coils[..*nobjs as usize]);
                ResponsePdu::ReadDiscreteInputs {
                    nobjs: *nobjs,
                    data: Data::coils(&coils[..res]),
                }
            }

            RequestPdu::ReadHoldingRegisters { nobjs, address } => {
                let res = self.read_registers(slave, func, *address, &mut regs[..*nobjs as usize]);
                ResponsePdu::ReadHoldingRegisters {
                    nobjs: *nobjs,
                    data: Data::registers(&regs[..res]),
                }
            }

            RequestPdu::ReadInputRegisters { nobjs, address } => {
                let res = self.read_registers(slave, func, *address, &mut regs[..*nobjs as usize]);
                ResponsePdu::ReadInputRegisters {
                    nobjs: *nobjs,
                    data: Data::registers(&regs[..res]),
                }
            }

            RequestPdu::WriteSingleCoil { address, value } => {
                self.write_coils(slave, 0x1, *address, &[*value]);
                ResponsePdu::WriteSingleCoil {
                    address: *address,
                    value: *value,
                }
            }

            RequestPdu::WriteSingleRegister { address, value } => {
                self.write_registers(slave, 0x3, *address, &[*value]);
                ResponsePdu::WriteSingleRegister {
                    address: *address,
                    value: *value,
                }
            }

            RequestPdu::WriteMultipleCoils {
                address,
                nobjs,
                data,
            } => {
                let count = *nobjs as usize;
                for i in 0..count {
                    coils[i] = data.get_bit(i).unwrap();
                }
                self.write_coils(slave, 0x1, *address, &coils[..count]);
                ResponsePdu::WriteMultipleCoils {
                    address: *address,
                    nobjs: *nobjs,
                }
            }

            RequestPdu::WriteMultipleRegisters {
                address,
                nobjs,
                data,
            } => {
                let count = *nobjs as usize;
                for i in 0..count {
                    regs[i] = data.get_u16(i).unwrap();
                }
                self.write_registers(slave, 0x3, *address, &regs[..count]);
                ResponsePdu::WriteMultipleRegisters {
                    address: *address,
                    nobjs: *nobjs,
                }
            }

            _ => ResponsePdu::Exception {
                function: func,
                code: Code::IllegalFunction,
            },
        };

        ResponseFrame::from_parts(request.id, request.slave, pdu)
    }

    pub fn new() -> Memory {
        Memory {
            values: std::collections::HashMap::new(),
        }
    }
}

fn usage() {
    println!(
        r#"slave-exchange [addresses]

Parameters:
    addresses - One or more addresses on which application should work

Env. variables:
    RUST_LOG - changes output verbosity. Values [error,warn,info,debug,trace]. info by default

Examples:
    slave-exchange - run with default parameters

    RUST_LOG=debug slave-exchange - run app with extended output

    slave-exchange tcp:0.0.0.0:8888 - run app on port 8888. TCP mode.

    slave-exchange tcp:0.0.0.0:1502 udp:0.0.0.0:1502 serial:/dev/ttyUSB0:9600-8-N-1 - run app on TCP/UDP ports #1502 and serial port /dev/ttyUSB0
    "#
    );
}

fn read_args() -> Vec<Settings> {
    env::args().skip(1).fold(Vec::new(), |mut acc, rec| {
        if let Ok(address) = TransportAddress::from_str(&rec) {
            let settings = Settings {
                address,
                ..Default::default()
            };
            acc.push(settings);
        }
        acc
    })
}

async fn wait_ctrl_c() {
    info!("press Ctrl+C to exit");
    let stop = signal::ctrl_c();
    tokio::select! {
        _ = stop => {
            info!("stopping...")
    }};
}

fn init_logger() {
    let mut builder = Builder::new();
    builder.filter_level(LevelFilter::Info);
    builder.parse_default_env();
    builder.init();
}

fn init_memory() -> Arc<Mutex<Memory>> {
    Arc::new(Mutex::new(Memory::new()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logger();

    let settings = read_args();

    if settings.is_empty() {
        usage();
    } else {
        let memory = init_memory();
        for record in settings {
            let local = memory.clone();
            builder::build_slave(record, move |request| {
                let mut locked = local.lock().unwrap();
                let answer = locked.process(&request.payload);
                Response::make(request, answer).try_send();
            })
            .await?;
        }
        wait_ctrl_c().await;
    }

    Ok(())
}
