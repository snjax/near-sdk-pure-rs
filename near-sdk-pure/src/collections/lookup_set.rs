//! A persistent set without iterators. Unlike `near_sdk_pure::collections::LookupSet` this set
//! doesn't store values separately in a vector, so it can't iterate over the values. But it
//! makes this implementation more efficient in the number of reads and writes.
use core::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};
use alloc::vec::Vec;

use crate::collections::append_slice;
use crate::env;

const ERR_ELEMENT_SERIALIZATION: &[u8] = b"Cannot serialize element with Borsh";

/// An non-iterable implementation of a set that stores its content directly on the trie.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct LookupSet<T> {
    element_prefix: Vec<u8>,
    #[borsh_skip]
    el: PhantomData<T>,
}

impl<T> LookupSet<T> {
    /// Create a new map. Use `element_prefix` as a unique prefix for trie keys.
    pub fn new(element_prefix: Vec<u8>) -> Self {
        Self { element_prefix, el: PhantomData }
    }

    fn raw_element_to_storage_key(&self, element_raw: &[u8]) -> Vec<u8> {
        append_slice(&self.element_prefix, element_raw)
    }

    /// Returns `true` if the serialized key is present in the map.
    fn contains_raw(&self, element_raw: &[u8]) -> bool {
        let storage_key = self.raw_element_to_storage_key(element_raw);
        env::storage_has_key(&storage_key)
    }

    /// Inserts a serialized element into the set.
    /// If the set did not have this value present, `true` is returned.
    /// If the set did have this value present, `false` is returned.
    pub fn insert_raw(&mut self, element_raw: &[u8]) -> bool {
        let storage_key = self.raw_element_to_storage_key(element_raw);
        !env::storage_write(&storage_key, b"")
    }

    /// Removes a serialized element from the set.
    /// Returns true if the element was present in the set.
    pub fn remove_raw(&mut self, element_raw: &[u8]) -> bool {
        let storage_key = self.raw_element_to_storage_key(element_raw);
        env::storage_remove(&storage_key)
    }
}

impl<T> LookupSet<T>
where
    T: BorshSerialize,
{
    fn serialize_element(element: &T) -> Vec<u8> {
        match element.try_to_vec() {
            Ok(x) => x,
            Err(_) => env::panic(ERR_ELEMENT_SERIALIZATION),
        }
    }

    /// Returns true if the set contains an element.
    pub fn contains(&self, element: &T) -> bool {
        self.contains_raw(&Self::serialize_element(element))
    }

    /// Removes a value from the set. Returns whether the value was present in the set.
    pub fn remove(&mut self, element: &T) -> bool {
        self.remove_raw(&Self::serialize_element(element))
    }

    /// Adds a value to the set.
    /// If the set did not have this value present, `true` is returned.
    /// If the set did have this value present, `false` is returned.
    pub fn insert(&mut self, element: &T) -> bool {
        self.insert_raw(&Self::serialize_element(element))
    }

    pub fn extend<IT: IntoIterator<Item = T>>(&mut self, iter: IT) {
        for el in iter {
            self.insert(&el);
        }
    }
}

