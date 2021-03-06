use near_sdk_pure::{near_bindgen, metadata};
use borsh::{BorshDeserialize, BorshSerialize};
metadata! {
#[near_bindgen]
#[derive(Default, BorshDeserialize, BorshSerialize)]
struct Incrementer {
    value: u32,
}

#[near_bindgen]
impl Incrementer {
    pub fn inc(&mut self, by: u32) {
        self.value += by;
    }
}
}

fn main() {}
