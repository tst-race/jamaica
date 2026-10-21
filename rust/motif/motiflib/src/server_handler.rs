// Copyright 2024 Two Six Technologies.

use crate::action::Action;

/// Trait for a struct that implements the ability to handle server requests.
/// A ServerHandler takes in some bytes and am address, and produces zero or many responses.
// TODO: conceptually doesn;t make sense since we can have many or no actions per incoming
//       payload. Need to rethink this, probably something queue/channel based since this will be threaded.
// TODO: conceptually a handler doesn't care about the address (right? validate this assumption).
//       It's really just being used as a handle so the router knows where to send responses.
//       Although if handler and connection is a one-to-one relationship then this can just be
//       mapped at a higher level.
pub trait ServerHandler {
    // fn get_next_action(&mut self, payload: &[u8], src: &std::net::SocketAddr) -> Action;

    fn handle_message(&self, payload: Vec<u8>, addr: &std::net::SocketAddr) -> bool;
}
