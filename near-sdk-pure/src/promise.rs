use borsh::BorshSchema;
use crate::types::{AccountId, Balance, Gas, PromiseIndex, PublicKey};
use core::cell::RefCell;
use crate::maybestd::collections::HashMap;
use crate::maybestd::io::{Error, Write};
use alloc::{rc::Rc, vec::Vec, vec};

pub enum PromiseAction {
    CreateAccount,
    DeployContract {
        code: Vec<u8>,
    },
    FunctionCall {
        method_name: Vec<u8>,
        arguments: Vec<u8>,
        amount: Balance,
        gas: Gas,
    },
    Transfer {
        amount: Balance,
    },
    Stake {
        amount: Balance,
        public_key: PublicKey,
    },
    AddFullAccessKey {
        public_key: PublicKey,
    },
    AddAccessKey {
        public_key: PublicKey,
        allowance: Balance,
        receiver_id: AccountId,
        method_names: Vec<u8>,
    },
    DeleteKey {
        public_key: PublicKey,
    },
    DeleteAccount {
        beneficiary_id: AccountId,
    },
}

impl PromiseAction {
    pub fn add(&self, promise_index: PromiseIndex) {
        use PromiseAction::*;
        match self {
            CreateAccount => crate::env::promise_batch_action_create_account(promise_index),
            DeployContract { code } => {
                crate::env::promise_batch_action_deploy_contract(promise_index, &code)
            }
            FunctionCall { method_name, arguments, amount, gas } => {
                crate::env::promise_batch_action_function_call(
                    promise_index,
                    &method_name,
                    &arguments,
                    *amount,
                    *gas,
                )
            }
            Transfer { amount } => {
                crate::env::promise_batch_action_transfer(promise_index, *amount)
            }
            Stake { amount, public_key } => {
                crate::env::promise_batch_action_stake(promise_index, *amount, public_key)
            }
            AddFullAccessKey { public_key } => {
                crate::env::promise_batch_action_add_key_with_full_access(
                    promise_index,
                    public_key,
                    0,
                )
            }
            AddAccessKey { public_key, allowance, receiver_id, method_names } => {
                crate::env::promise_batch_action_add_key_with_function_call(
                    promise_index,
                    public_key,
                    0,
                    *allowance,
                    receiver_id,
                    &method_names,
                )
            }
            DeleteKey { public_key } => {
                crate::env::promise_batch_action_delete_key(promise_index, public_key)
            }
            DeleteAccount { beneficiary_id } => {
                crate::env::promise_batch_action_delete_account(promise_index, beneficiary_id)
            }
        }
    }
}

pub struct PromiseSingle {
    pub account_id: AccountId,
    pub actions: RefCell<Vec<PromiseAction>>,
    pub after: RefCell<Option<Promise>>,
    /// Promise index that is computed only once.
    pub promise_index: RefCell<Option<PromiseIndex>>,
}

impl PromiseSingle {
    pub fn construct_recursively(&self) -> PromiseIndex {
        let mut promise_lock = self.promise_index.borrow_mut();
        if let Some(res) = promise_lock.as_ref() {
            return *res;
        }
        let promise_index = if let Some(after) = self.after.borrow().as_ref() {
            crate::env::promise_batch_then(after.construct_recursively(), &self.account_id)
        } else {
            crate::env::promise_batch_create(&self.account_id)
        };
        let actions_lock = self.actions.borrow();
        for action in actions_lock.iter() {
            action.add(promise_index);
        }
        *promise_lock = Some(promise_index);
        promise_index
    }
}

pub struct PromiseJoint {
    pub promise_a: Promise,
    pub promise_b: Promise,
    /// Promise index that is computed only once.
    pub promise_index: RefCell<Option<PromiseIndex>>,
}

impl PromiseJoint {
    pub fn construct_recursively(&self) -> PromiseIndex {
        let mut promise_lock = self.promise_index.borrow_mut();
        if let Some(res) = promise_lock.as_ref() {
            return *res;
        }
        let res = crate::env::promise_and(&[
            self.promise_a.construct_recursively(),
            self.promise_b.construct_recursively(),
        ]);
        *promise_lock = Some(res);
        res
    }
}


#[derive(Clone)]
pub struct Promise {
    subtype: PromiseSubtype,
    should_return: RefCell<bool>,
}

/// Until we implement strongly typed promises we serialize them as unit struct.
impl BorshSchema for Promise {
    fn add_definitions_recursively(
        definitions: &mut HashMap<borsh::schema::Declaration, borsh::schema::Definition>,
    ) {
        <()>::add_definitions_recursively(definitions);
    }

    fn declaration() -> borsh::schema::Declaration {
        <()>::declaration()
    }
}

#[derive(Clone)]
pub enum PromiseSubtype {
    Single(Rc<PromiseSingle>),
    Joint(Rc<PromiseJoint>),
}

impl Promise {
    /// Create a promise that acts on the given account.
    pub fn new(account_id: AccountId) -> Self {
        Self {
            subtype: PromiseSubtype::Single(Rc::new(PromiseSingle {
                account_id,
                actions: RefCell::new(vec![]),
                after: RefCell::new(None),
                promise_index: RefCell::new(None),
            })),
            should_return: RefCell::new(false),
        }
    }

    fn add_action(self, action: PromiseAction) -> Self {
        match &self.subtype {
            PromiseSubtype::Single(x) => x.actions.borrow_mut().push(action),
            PromiseSubtype::Joint(_) => panic!("Cannot add action to a joint promise."),
        }
        self
    }

    /// Create account on which this promise acts.
    pub fn create_account(self) -> Self {
        self.add_action(PromiseAction::CreateAccount)
    }

    /// Deploy a smart contract to the account on which this promise acts.
    pub fn deploy_contract(self, code: Vec<u8>) -> Self {
        self.add_action(PromiseAction::DeployContract { code })
    }

    /// A low-level interface for making a function call to the account that this promise acts on.
    pub fn function_call(
        self,
        method_name: Vec<u8>,
        arguments: Vec<u8>,
        amount: Balance,
        gas: Gas,
    ) -> Self {
        self.add_action(PromiseAction::FunctionCall { method_name, arguments, amount, gas })
    }

    /// Transfer tokens to the account that this promise acts on.
    pub fn transfer(self, amount: Balance) -> Self {
        self.add_action(PromiseAction::Transfer { amount })
    }

    /// Stake the account for the given amount of tokens using the given public key.
    pub fn stake(self, amount: Balance, public_key: PublicKey) -> Self {
        self.add_action(PromiseAction::Stake { amount, public_key })
    }

    /// Add full access key to the given account.
    pub fn add_full_access_key(self, public_key: PublicKey) -> Self {
        self.add_action(PromiseAction::AddFullAccessKey { public_key })
    }

    /// Add an access key that is restricted to only calling a smart contract on some account using
    /// only a restricted set of methods. Here `method_names` is a comma separated list of methods,
    /// e.g. `b"method_a,method_b"`.
    pub fn add_access_key(
        self,
        public_key: PublicKey,
        allowance: Balance,
        receiver_id: AccountId,
        method_names: Vec<u8>,
    ) -> Self {
        self.add_action(PromiseAction::AddAccessKey {
            public_key,
            allowance,
            receiver_id,
            method_names,
        })
    }

    /// Delete access key from the given account.
    pub fn delete_key(self, public_key: PublicKey) -> Self {
        self.add_action(PromiseAction::DeleteKey { public_key })
    }

    /// Delete the given account.
    pub fn delete_account(self, beneficiary_id: AccountId) -> Self {
        self.add_action(PromiseAction::DeleteAccount { beneficiary_id })
    }


    pub fn and(self, other: Promise) -> Promise {
        Promise {
            subtype: PromiseSubtype::Joint(Rc::new(PromiseJoint {
                promise_a: self,
                promise_b: other,
                promise_index: RefCell::new(None),
            })),
            should_return: RefCell::new(false),
        }
    }


    pub fn then(self, mut other: Promise) -> Promise {
        match &mut other.subtype {
            PromiseSubtype::Single(x) => *x.after.borrow_mut() = Some(self),
            PromiseSubtype::Joint(_) => panic!("Cannot callback joint promise."),
        }
        other
    }

    /// A specialized, relatively low-level API method. Allows to mark the given promise as the one
    /// that should be considered as a return value.
    ///
    /// In the below code `a1` and `a2` functions are equivalent.
    /// ```
    /// # use near_sdk_pure::{ext_contract, near_bindgen, Promise};
    /// # use borsh::{BorshDeserialize, BorshSerialize};
    /// #[ext_contract]
    /// pub trait ContractB {
    ///     fn b(&mut self);
    /// }
    ///
    /// #[near_bindgen]
    /// #[derive(Default, BorshDeserialize, BorshSerialize)]
    /// struct ContractA {}
    ///
    /// #[near_bindgen]
    /// impl ContractA {
    ///     pub fn a1(&self) {
    ///        contract_b::b(&"bob_near".to_string(), 0, 1_000).as_return();
    ///     }
    ///
    ///     pub fn a2(&self) -> Promise {
    ///        contract_b::b(&"bob_near".to_string(), 0, 1_000)
    ///     }
    /// }
    /// ```
    pub fn as_return(self) -> Self {
        *self.should_return.borrow_mut() = true;
        self
    }

    fn construct_recursively(&self) -> PromiseIndex {
        let res = match &self.subtype {
            PromiseSubtype::Single(x) => x.construct_recursively(),
            PromiseSubtype::Joint(x) => x.construct_recursively(),
        };
        if *self.should_return.borrow() {
            crate::env::promise_return(res);
        }
        res
    }
}

impl Drop for Promise {
    fn drop(&mut self) {
        self.construct_recursively();
    }
}

impl serde::Serialize for Promise {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        *self.should_return.borrow_mut() = true;
        serializer.serialize_unit()
    }
}

pub enum PromiseOrValue<T> {
    Promise(Promise),
    Value(T),
}

impl<T> BorshSchema for PromiseOrValue<T>
where
    T: BorshSchema,
{
    fn add_definitions_recursively(
        definitions: &mut HashMap<borsh::schema::Declaration, borsh::schema::Definition>,
    ) {
        T::add_definitions_recursively(definitions);
    }

    fn declaration() -> borsh::schema::Declaration {
        T::declaration()
    }
}

impl<T> From<Promise> for PromiseOrValue<T> {
    fn from(promise: Promise) -> Self {
        PromiseOrValue::Promise(promise.as_return())
    }
}

impl<T: serde::Serialize> serde::Serialize for PromiseOrValue<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            // Only actual value is serialized.
            PromiseOrValue::Value(x) => x.serialize(serializer),
            // The promise is dropped to cause env::promise calls.
            PromiseOrValue::Promise(_) => serializer.serialize_unit(),
        }
    }
}

impl<T: borsh::BorshSerialize> borsh::BorshSerialize for PromiseOrValue<T> {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            // Only actual value is serialized.
            PromiseOrValue::Value(x) => x.serialize(writer),
            // The promise is dropped to cause env::promise calls.
            PromiseOrValue::Promise(_) => Ok(()),
        }
    }
}
