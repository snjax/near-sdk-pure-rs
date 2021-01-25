#![no_std]
extern crate alloc;


#[cfg(test)]
extern crate quickcheck;


pub mod maybestd {
    pub use borsh::maybestd::*;
    pub use core::panic;
}


pub use near_sdk_pure_macros::{
    callback, callback_vec, ext_contract, init, metadata, near_bindgen, result_serializer,
    serializer, PanicOnDefault,
};

pub mod collections;
mod environment;
pub use environment::env;

mod promise;
pub use promise::{Promise, PromiseOrValue};

mod metadata;
pub use metadata::{Metadata, MethodMetadata};

pub mod json_types;
pub mod types;

pub use types::*;


pub mod utils;
pub use crate::utils::*;

pub use environment::blockchain_interface::BlockchainInterface;

pub mod test_utils;

// Exporting common crates

#[doc(hidden)]
pub use borsh;

#[doc(hidden)]
pub use base64;

#[doc(hidden)]
pub use bs58;

#[doc(hidden)]
pub use serde;

#[doc(hidden)]
pub use serde_json;

#[doc(hidden)]
pub use wee_alloc;
