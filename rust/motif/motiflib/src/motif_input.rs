// Copyright 2024 Two Six Technologies.

use crate::action::Action;

pub trait MotifInput {
    fn get_next_action(&mut self) -> Action;
}
