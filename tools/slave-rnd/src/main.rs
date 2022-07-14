use env_logger::Builder;
use frame::exception::Code;
use frame::request::{RequestFrame, RequestPDU};
use frame::response::{ResponseFrame, ResponsePDU};
use frame::{MAX_NCOILS, MAX_NREGS};
use futures::{Stream, StreamExt};
use log::{info, LevelFilter};
use tokio::signal;

use rand::Rng;
use std::env;
use std::str::FromStr;
use transport::builder;
use transport::{settings::Settings, settings::TransportAddress};

extern crate frame;
extern crate transport;

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

fn make_answer(request: &RequestFrame) -> ResponseFrame {
    let slave = request.slave;

    let mut registers = [0u16; MAX_NREGS];
    let mut coils = [false; MAX_NCOILS];

    let pdu = match &request.pdu {
        RequestPDU::ReadCoils { nobjs, .. } => {
            let nobjs = *nobjs as usize;
            fill_coils(&mut coils[0..nobjs]);
            ResponsePDU::read_coils(&coils[0..nobjs])
        }

        RequestPDU::ReadDiscreteInputs { nobjs, .. } => {
            let nobjs = *nobjs as usize;
            fill_coils(&mut coils[0..nobjs]);
            ResponsePDU::read_discrete_inputs(&coils[0..nobjs as usize])
        }

        RequestPDU::ReadHoldingRegisters { nobjs, .. } => {
            let nobjs = *nobjs as usize;
            fill_registers(&mut registers[0..nobjs]);
            ResponsePDU::read_holding_registers(&registers[0..nobjs])
        }

        RequestPDU::ReadInputRegisters { nobjs, .. } => {
            let nobjs = *nobjs as usize;
            fill_registers(&mut registers[0..nobjs]);
            ResponsePDU::read_input_registers(&registers[0..nobjs as usize])
        }

        RequestPDU::WriteSingleCoil { address, value } => {
            ResponsePDU::write_single_coil(*address, *value)
        }

        RequestPDU::WriteSingleRegister { address, value } => {
            ResponsePDU::write_single_register(*address, *value)
        }

        RequestPDU::WriteMultipleCoils { address, nobjs, .. } => {
            ResponsePDU::write_multiple_coils(*address, *nobjs)
        }

        RequestPDU::WriteMultipleRegisters { address, nobjs, .. } => {
            ResponsePDU::write_multiple_registers(*address, *nobjs)
        }

        RequestPDU::EncapsulatedInterfaceTransport { mei_type, data, .. } => {
            match (mei_type, data.get_u8(0)) {
                (0xE, Some(0) | Some(1) | Some(2)) => {
                    ResponsePDU::encapsulated_interface_transport(
                        *mei_type,
                        "modbus-imit".as_bytes(),
                    )
                }
                _ => ResponsePDU::exception(0x2b, Code::IllegalDataValue),
            }
        }

        RequestPDU::Raw { function, .. } => {
            ResponsePDU::exception(*function, Code::IllegalFunction)
        }
    };
    ResponseFrame::rtu(slave, pdu)
}

fn read_args() -> Option<Settings> {
    let arg: String = env::args().skip(1).take(1).collect();

    if arg == "--help" || arg == "-h" {
        println!(
            r#"slave_rnd [address]

Parameters:
    address - optional parameter for binding server socket. 0.0.0.0:502 by default

Env. variables:
    RUST_LOG - changes output verbosity. Values [error,warn,info,debug,trace]. info by default

Examples:
    slave_rnd - run with default parameters

    RUST_LOG=debug slave_rnd - run app with extended output

    slave_rnd 0.0.0.0:8888 - run app on port 8888"#
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

fn init_processor<S>(mut input: S)
where
    S: Stream<Item = transport::Request> + std::marker::Unpin + std::marker::Send + 'static,
{
    info!("start message processor");
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(request) = input.next() => {
                    request.response(make_answer(&request.request)).await;
                }
            }
        }
    });
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(settings) = read_args() {
        init_logger();
        let input = builder::build(settings).await?;
        init_processor(input);
        wait_ctrl_c().await;
    }
    Ok(())
}
