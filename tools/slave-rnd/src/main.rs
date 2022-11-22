extern crate frame;
extern crate transport;

use env_logger::Builder;
use frame::exception::Code;
use frame::{RequestFrame, RequestPdu, ResponseFrame, ResponsePdu, MAX_NCOILS, MAX_NREGS};
use futures::{Stream, StreamExt};
use log::{info, LevelFilter};
use tokio::signal;

use rand::Rng;
use std::env;
use std::str::FromStr;
use transport::builder;
use transport::{settings::Settings, settings::TransportAddress, Response};

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
            ResponsePdu::read_discrete_inputs(&coils[0..nobjs as usize])
        }

        RequestPdu::ReadHoldingRegisters { nobjs, .. } => {
            let nobjs = *nobjs as usize;
            fill_registers(&mut registers[0..nobjs]);
            ResponsePdu::read_holding_registers(&registers[0..nobjs])
        }

        RequestPdu::ReadInputRegisters { nobjs, .. } => {
            let nobjs = *nobjs as usize;
            fill_registers(&mut registers[0..nobjs]);
            ResponsePdu::read_input_registers(&registers[0..nobjs as usize])
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

    ResponseFrame::new(request.slave, pdu)
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

fn init_processor<S>(mut input: S)
where
    S: Stream<Item = transport::Request> + std::marker::Unpin + std::marker::Send + 'static,
{
    info!("start message processor");
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(request) = input.next() => {
                    let answer = make_answer(&request.payload);
                    Response::make(
                        request,
                        answer
                    ).send().await;
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
