// Copyright 2024 Two Six Technologies.

use std::fmt::Debug;
use std::rc::Rc;

// TODO: !!!!!!!!!!! use a trait, not a specific struct. this is just POC
use crate::quic_client::QuicClient; // TODO: or maybe just delete this and capture in the closure.

type Payload = Vec<u8>;
///
/// Function to dynamically generate a payload to send for a SendAction. Takes in a list of the
/// previously received payloads.
///
type SendActionCreateFunc = Rc<dyn Fn(&Vec<Payload>) -> Payload>;

#[derive(Clone)]
pub struct SendAction {
    // pub payload: Vec<u8>,
    pub create: SendActionCreateFunc,
}

impl Debug for SendAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ SendAction }}")
        // write!(f, "{{ SendAction payload: {:?} }}", self.payload);
        // Ok(())
    }
}

impl SendAction {
    pub fn new(create: SendActionCreateFunc) -> Self {
        Self { create }
        // Self { payload, create }
    }
}

impl Default for SendAction {
    fn default() -> Self {
        Self {
            // payload: vec![],
            create: Rc::new(|x| {
                println!("closure! {:?}", x.len());
                vec![0]
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReceiveAction {
    pub timeout: i32,
}

impl ReceiveAction {
    pub fn new(timeout: i32) -> Self {
        Self { timeout }
    }
}

impl Default for ReceiveAction {
    fn default() -> Self {
        Self { timeout: 0 }
    }
}

#[derive(Clone, Debug)]
pub enum Action {
    Send(SendAction),
    Receive(ReceiveAction),
    Quit,
}
