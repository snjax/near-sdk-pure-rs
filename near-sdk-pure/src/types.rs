use serde::{Serialize, Deserialize};
use alloc::{string::String, vec::Vec};


pub type AccountId = String;
pub type PublicKey = Vec<u8>;
pub type BlockHeight = u64;
pub type EpochHeight = u64;
pub type Balance = u128;
pub type Gas = u64;
pub type PromiseIndex = u64;
pub type ReceiptIndex = u64;
pub type IteratorIndex = u64;
pub type StorageUsage = u64;
pub type ProtocolVersion = u32;


pub mod bytes_as_str {
    use super::*;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(arr: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&String::from_utf8(arr.clone()).unwrap())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(s.into_bytes())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum ReturnData {
    /// Method returned some value or data.
    #[serde(with = "crate::types::bytes_as_str")]
    Value(Vec<u8>),

    /// The return value of the method should be taken from the return value of another method
    /// identified through receipt index.
    ReceiptIndex(ReceiptIndex),

    /// Method hasn't returned any data or promise.
    None,
}

/// When there is a callback attached to one or more contract calls the execution results of these
/// calls are available to the contract invoked through the callback.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum PromiseResult {
    /// Current version of the protocol never returns `PromiseResult::NotReady`.
    NotReady,
    #[serde(with = "crate::types::bytes_as_str")]
    Successful(Vec<u8>),
    Failed,
}

