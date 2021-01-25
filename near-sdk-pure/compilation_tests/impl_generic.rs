//! Impl block has type parameters.

use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk_pure::near_bindgen;
use core::marker::PhantomData;

#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer<T> {
    value: u32,
    data: PhantomData<T>,
}

#[near_bindgen]
impl<'a, T: 'a + core::fmt::Display> Incrementer<T> {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}

fn main() {}
