use modbus::data::prelude::*;
use modbus::frame::exception::Code;
use modbus::frame::prelude::*;
use modbus::transport::builder;
use modbus::transport::prelude::*;

use env_logger::Builder;
use log::{info, warn, LevelFilter};
use tokio::signal;

use rand::Rng;
use std::env;
use std::str::FromStr;

fn fill_registers(registers: &mut [u16]) {
    for item in registers.iter_mut() {
        *item = rand::thread_rng().gen();
    }
}

fn fill_coils(coils: &mut [bool]) {
    for item in coils.iter_mut() {
        *item = rand::thread_rng().gen();
    }
}

fn make_answer(request: Request) -> Response {
    let mut registers = [0u16; MAX_NREGS];
    let mut coils = [false; MAX_NCOILS];
    let pdu = match &request.pdu {
        RequestPdu::ReadCoils { nobjs, .. } => {
            let nobjs = *nobjs as usize;
            fill_coils(&mut coils[0..nobjs]);
            ResponsePdu::read_coils(&coils[0..nobjs])
        }

        RequestPdu::ReadDiscreteInputs { nobjs, .. } => {
            let nobjs = *nobjs as usize;
            fill_coils(&mut coils[0..nobjs]);
            ResponsePdu::read_discrete_inputs(&coils[0..nobjs])
        }

        RequestPdu::ReadHoldingRegisters { nobjs, .. } => {
            let nobjs = *nobjs as usize;
            fill_registers(&mut registers[0..nobjs]);
            ResponsePdu::read_holding_registers(&registers[0..nobjs])
        }

        RequestPdu::ReadInputRegisters { nobjs, .. } => {
            let nobjs = *nobjs as usize;
            fill_registers(&mut registers[0..nobjs]);
            ResponsePdu::read_input_registers(&registers[0..nobjs])
        }

        RequestPdu::WriteSingleCoil { address, value } => {
            ResponsePdu::write_single_coil(*address, *value)
        }

        RequestPdu::WriteSingleRegister { address, value } => {
            ResponsePdu::write_single_register(*address, *value)
        }

        RequestPdu::WriteMultipleCoils { address, nobjs, .. } => {
            ResponsePdu::write_multiple_coils(*address, *nobjs)
        }

        RequestPdu::WriteMultipleRegisters { address, nobjs, .. } => {
            ResponsePdu::write_multiple_registers(*address, *nobjs)
        }

        RequestPdu::EncapsulatedInterfaceTransport { mei_type, data, .. } => {
            match (mei_type, data.get_u8(0)) {
                (0xE, Some(0) | Some(1) | Some(2)) => {
                    ResponsePdu::encapsulated_interface_transport(
                        *mei_type,
                        "modbus-imit".as_bytes(),
                    )
                }
                _ => ResponsePdu::exception(0x2b, Code::IllegalDataValue),
            }
        }

        RequestPdu::Raw { function, .. } => {
            ResponsePdu::exception(*function, Code::IllegalFunction)
        }
    };

    Response::make(request, pdu)
}

fn read_args() -> Option<Settings> {
    let arg: String = env::args().skip(1).take(1).collect();

    if arg == "--help" || arg == "-h" {
        println!(
            r#"slave-rnd [address]

Parameters:
    address - optional parameter for binding server socket. 0.0.0.0:502 by default

Env. variables:
    RUST_LOG - changes output verbosity. Values [error,warn,info,debug,trace]. info by default

Examples:
    slave-rnd - run with default parameters

    RUST_LOG=debug slave-rnd - run app with extended output

    slave-rnd tcp:0.0.0.0:8888 - run app on port 8888. TCP mode.

    slave-rnd udp:0.0.0.0:8888 - run app on port 8888. UDP mode.

    slave-rnd serial:/dev/ttyUSB0:19200-8-E-1 - run app on serial port. RTU mode.
    "#
        );
        None
    } else {
        let mut settings = Settings::default();
        if !arg.is_empty() {
            settings.address = TransportAddress::from_str(&arg).unwrap();
        }
        Some(settings)
    }
}

async fn wait_ctrl_c() {
    info!("press ctrl+c to exit");
    let _ = signal::ctrl_c().await;
    info!("stopping...");
}

fn init_logger() {
    Builder::new()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(settings) = read_args() {
        init_logger();
        builder::build_slave(settings, |request| {
            let _ = make_answer(request).send().map_err(|e| warn!("{:?}", e));
        })
        .await?;
        wait_ctrl_c().await;
    }
    Ok(())
}
