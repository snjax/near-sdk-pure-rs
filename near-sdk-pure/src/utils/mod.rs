use crate::AccountId;
use alloc::{string::{String, ToString}, vec::Vec};

#[macro_export]
macro_rules! log {
    ($arg:tt) => {
        crate::env::log($arg.as_bytes())
    };
    ($($arg:tt)*) => {
        crate::env::log(alloc::format!($($arg)*).as_bytes())
    };
}

#[derive(Debug)]
pub struct PendingContractTx {
    pub receiver_id: AccountId,
    pub method: String,
    pub args: Vec<u8>,
    pub is_view: bool,
}

impl PendingContractTx {
    pub fn new(receiver_id: &str, method: &str, args: serde_json::Value, is_view: bool) -> Self {
        Self {
            receiver_id: receiver_id.to_string(),
            method: method.to_string(),
            args: args.to_string().into_bytes(),
            is_view,
        }
    }
}

