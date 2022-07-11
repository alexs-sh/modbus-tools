use env_logger::Builder;
use frame::exception::Code;
use frame::request::{RequestFrame, RequestPDU};
use frame::response::{ResponseFrame, ResponsePDU};
use frame::{MAX_NCOILS, MAX_NREGS};
use log::{info, LevelFilter};
use tokio::signal;

use rand::Rng;
use std::env;
use transport::tcp::server::{TcpServer, TcpServerHandler, TcpServerSettings};

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

    let pdu = match request.pdu {
        RequestPDU::ReadCoils { nobjs, .. } => {
            let nobjs = nobjs as usize;
            fill_coils(&mut coils[0..nobjs]);
            ResponsePDU::read_coils(&coils[0..nobjs])
        }

        RequestPDU::ReadDiscreteInputs { nobjs, .. } => {
            let nobjs = nobjs as usize;
            fill_coils(&mut coils[0..nobjs]);
            ResponsePDU::read_discrete_inputs(&coils[0..nobjs as usize])
        }

        RequestPDU::ReadHoldingRegisters { nobjs, .. } => {
            let nobjs = nobjs as usize;
            fill_registers(&mut registers[0..nobjs]);
            ResponsePDU::read_holding_registers(&registers[0..nobjs])
        }

        RequestPDU::ReadInputRegisters { nobjs, .. } => {
            let nobjs = nobjs as usize;
            fill_registers(&mut registers[0..nobjs]);
            ResponsePDU::read_input_registers(&registers[0..nobjs as usize])
        }

        RequestPDU::WriteSingleCoil { address, value } => {
            ResponsePDU::write_single_coil(address, value)
        }

        RequestPDU::WriteSingleRegister { address, value } => {
            ResponsePDU::write_single_register(address, value)
        }

        RequestPDU::WriteMultipleCoils { address, nobjs, .. } => {
            ResponsePDU::write_multiple_coils(address, nobjs)
        }
        RequestPDU::WriteMultipleRegisters { address, nobjs, .. } => {
            ResponsePDU::write_multiple_registers(address, nobjs)
        }

        RequestPDU::Raw { function, .. } => ResponsePDU::exception(function, Code::IllegalFunction),
    };
    ResponseFrame::rtu(slave, pdu)
}

fn read_args() -> Option<TcpServerSettings> {
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
        let mut settings = TcpServerSettings::default();
        if !arg.is_empty() {
            settings.address = arg;
        }
        Some(settings)
    }
}

fn init_processor(mut server: TcpServerHandler) {
    info!("start message processor");
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(request) = server.request_rx.recv() => {
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

async fn init_server(settings: TcpServerSettings) -> Result<TcpServerHandler, std::io::Error> {
    info!("start server {}", settings.address);
    TcpServer::build(settings.clone()).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(settings) = read_args() {
        init_logger();
        let server = init_server(settings).await?;
        init_processor(server);
        wait_ctrl_c().await;
    }
    Ok(())
}
