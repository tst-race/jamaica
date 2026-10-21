// Copyright 2024 Two Six Technologies.

use std::net::SocketAddr;
use std::sync::mpsc::{RecvTimeoutError, SendError};
use std::time::Duration;

pub trait NetworkComm {
    fn send(&self, payload: Vec<u8>) -> Result<(), SendError<Vec<u8>>>;
    fn receive(&self, timeout: Duration) -> Result<(Vec<u8>, SocketAddr), RecvTimeoutError>;
    /// TODO: not sure we'll actually need this, a la IIRC.
    fn close(&self) -> std::io::Result<()>;
}
