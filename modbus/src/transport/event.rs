use super::{Request, Response};
use crate::frame::prelude::*;
use log::{debug, error, info, trace, warn};
use std::fmt::Debug;

#[derive(Debug)]
enum Event<'a> {
    Input(&'a dyn Debug, &'a [u8]),
    Output(&'a dyn Debug, &'a [u8]),
    Request(&'a dyn Debug, u128, &'a u8, &'a RequestPdu),
    Response(&'a dyn Debug, u128, &'a u8, &'a ResponsePdu),
    Error(&'a dyn Debug, &'a dyn Debug),
    Warning(&'a dyn Debug, &'a dyn Debug),
    Info(&'a dyn Debug, &'a dyn Debug),
}

pub(crate) struct EventLog {}

impl EventLog {
    pub fn input(name: &dyn Debug, data: &[u8]) {
        let event = Event::Input(&name, data);
        trace!("{:?}", event);
    }

    pub fn output(name: &dyn Debug, data: &[u8]) {
        let event = Event::Output(&name, data);
        trace!("{:?}", event);
    }

    pub fn request(name: &dyn Debug, msg: &Request) {
        let event = Event::Request(&name, msg.uuid.as_u128(), &msg.slave, &msg.pdu);
        debug!("{:?}", event);
    }

    pub fn response(name: &dyn Debug, msg: &Response) {
        let event = Event::Response(&name, msg.uuid.as_u128(), &msg.slave, &msg.pdu);
        debug!("{:?}", event);
    }

    pub fn warning(name: &dyn Debug, warn: &dyn Debug) {
        let event = Event::Warning(&name, warn);
        warn!("{:?}", event);
    }

    pub fn error(name: &dyn Debug, err: &dyn Debug) {
        let event = Event::Error(&name, err);
        error!("{:?}", event);
    }

    pub fn info(name: &dyn Debug, err: &dyn Debug) {
        let event = Event::Info(&name, err);
        info!("{:?}", event);
    }
}
