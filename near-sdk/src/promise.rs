use borsh::BorshSchema;
use std::collections::HashMap;
use std::marker::PhantomData;

use crate::{AccountId, Balance, Gas, GasWeight, PromiseIndex, PublicKey};

/// This type indicates that the promise will not return any bytes.
// TODO this is likely broken. I believe JSON schema treats as any, but borsh may try to deserialize 0 bytes
pub type NoReturn = ();

mod private {
    /// Types allowed to be used as a promise are sealed to ensure only Promise types can be used.
    pub trait Sealed {}

    impl<'a, T> Sealed for super::PromiseBuilder<'a, T> {}
    impl<'a, T> Sealed for super::PromiseThen<'a, T> {}
    impl<L, R> Sealed for super::PromiseAnd<L, R> {}
}

/// Types that can be scheduled through [`PromiseBuilder::and`] and other promise equivalents.
pub trait SchedulablePromise<T>: private::Sealed {
    fn append_to_promises(self, promises: &mut Vec<PromiseIndex>);
}

impl<'a, T> SchedulablePromise<T> for PromiseBuilder<'a, T> {
    fn append_to_promises(self, promises: &mut Vec<PromiseIndex>) {
        promises.push(self.schedule())
    }
}
impl<'a, T> SchedulablePromise<T> for PromiseThen<'a, T> {
    fn append_to_promises(self, promises: &mut Vec<PromiseIndex>) {
        promises.push(self.schedule())
    }
}
impl<L, R> SchedulablePromise<PromiseAnd<L, R>> for PromiseAnd<L, R> {
    fn append_to_promises(self, promises: &mut Vec<PromiseIndex>) {
        promises.extend(self.promises)
    }
}

/// A structure representing a result of the scheduled execution on another contract.
///
/// Smart contract developers will explicitly use `Promise` in two situations:
/// * When they need to return `Promise`.
///
///   In the following code if someone calls method `ContractA::a` they will internally cause an
///   execution of method `ContractB::b` of `bob_near` account, and the return value of `ContractA::a`
///   will be what `ContractB::b` returned.
/// ```no_run
/// # use near_sdk::{ext_contract, near_bindgen, Promise, Gas};
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
///     pub fn a(&self) -> Promise {
///         contract_b::ext(&"bob_near".parse().unwrap()).b().schedule_as_return()
///     }
/// }
/// ```
///
/// * When they need to create a transaction with one or many actions, e.g. the following code
///   schedules a transaction that creates an account, transfers tokens, and assigns a public key:
///
/// ```no_run
/// # use near_sdk::{Promise, env, test_utils::VMContextBuilder, testing_env};
/// # testing_env!(VMContextBuilder::new().signer_account_id("bob_near".parse().unwrap())
/// #               .account_balance(1000).prepaid_gas(1_000_000.into()).build());
/// Promise::new(&"bob_near".parse().unwrap())
///   .create_account()
///   .transfer(1000)
///   .add_full_access_key(&env::signer_account_pk())
///   .schedule();
/// ```
#[must_use]
#[derive(Debug)]
pub struct PromiseBuilder<'a, T = NoReturn> {
    account_id: &'a AccountId,
    actions: Vec<PromiseAction<'a>>,
    _marker: PhantomData<fn() -> T>,
}

impl<'a> PromiseBuilder<'a, NoReturn> {
    /// Create a promise that acts on the given account.
    pub fn new(account_id: &'a AccountId) -> Self {
        Self { account_id, actions: Vec::new(), _marker: Default::default() }
    }
}

impl<'a, T> PromiseBuilder<'a, T> {
    fn add_action(mut self, action: PromiseAction<'a>) -> Self {
        self.actions.push(action);
        self
    }

    /// Create account on which this promise acts.
    pub fn create_account(self) -> Self {
        self.add_action(PromiseAction::CreateAccount)
    }

    /// Deploy a smart contract to the account on which this promise acts.
    pub fn deploy_contract(self, code: &'a [u8]) -> Self {
        self.add_action(PromiseAction::DeployContract { code })
    }

    /// A low-level interface for making a function call to the account that this promise acts on.
    pub fn function_call<F>(
        self,
        function_name: &'a str,
        // TODO should this be part of opts or have some convenience method for serialization
        arguments: Vec<u8>,
        opts: FunctionCallOpts,
    ) -> PromiseBuilder<'a, F> {
        let Self { account_id, actions, _marker: _ } =
            self.add_action(PromiseAction::FunctionCallWeight {
                function_name,
                arguments,
                amount: opts.deposit.unwrap_or_default(),
                gas: opts.static_gas.unwrap_or_default(),
                weight: opts.gas_weight.unwrap_or(GasWeight(1)),
            });
        PromiseBuilder { account_id, actions, _marker: Default::default() }
    }

    /// Transfer tokens to the account that this promise acts on.
    pub fn transfer(self, amount: Balance) -> Self {
        self.add_action(PromiseAction::Transfer { amount })
    }

    /// Stake the account for the given amount of tokens using the given public key.
    pub fn stake(self, amount: Balance, public_key: &'a PublicKey) -> Self {
        self.add_action(PromiseAction::Stake { amount, public_key })
    }

    /// Add full access key to the given account.
    pub fn add_full_access_key(self, public_key: &'a PublicKey) -> Self {
        self.add_full_access_key_with_nonce(public_key, 0)
    }

    /// Add full access key to the given account with a provided nonce.
    pub fn add_full_access_key_with_nonce(self, public_key: &'a PublicKey, nonce: u64) -> Self {
        self.add_action(PromiseAction::AddFullAccessKey { public_key, nonce })
    }

    /// Add an access key that is restricted to only calling a smart contract on some account using
    /// only a restricted set of methods. Here `function_names` is a comma separated list of methods,
    /// e.g. `"method_a,method_b".to_string()`.
    pub fn add_access_key(
        self,
        public_key: &'a PublicKey,
        allowance: Balance,
        receiver_id: &'a AccountId,
        function_names: &'a str,
    ) -> Self {
        self.add_access_key_with_nonce(public_key, allowance, receiver_id, function_names, 0)
    }

    /// Add an access key with a provided nonce.
    pub fn add_access_key_with_nonce(
        self,
        public_key: &'a PublicKey,
        allowance: Balance,
        receiver_id: &'a AccountId,
        function_names: &'a str,
        nonce: u64,
    ) -> Self {
        self.add_action(PromiseAction::AddAccessKey {
            public_key,
            allowance,
            receiver_id,
            function_names,
            nonce,
        })
    }

    /// Delete access key from the given account.
    pub fn delete_key(self, public_key: &'a PublicKey) -> Self {
        self.add_action(PromiseAction::DeleteKey { public_key })
    }

    /// Delete the given account.
    pub fn delete_account(self, beneficiary_id: &'a AccountId) -> Self {
        self.add_action(PromiseAction::DeleteAccount { beneficiary_id })
    }

    /// Merge this promise with another promise, so that we can schedule execution of another
    /// smart contract right after all merged promises finish.
    ///
    /// ```no_run
    /// # use near_sdk::{Promise, testing_env};
    /// let bob = "bob.near".parse().unwrap();
    /// let carol = "carol.near".parse().unwrap();
    /// let p3 = Promise::new(&bob).create_account().and(Promise::new(&carol).create_account());
    /// p3.schedule();
    /// ```
    pub fn and<O>(self, other: impl SchedulablePromise<O>) -> PromiseAnd<T, O> {
        let mut promises = vec![self.schedule()];
        other.append_to_promises(&mut promises);
        PromiseAnd { promises, _marker: Default::default() }
    }

    /// Schedules execution of another promise right after the current promise finish executing.
    ///
    /// In the following code `bob.near` and `dave.near` will be created concurrently. `carol.near`
    /// creation will wait for `bob.near` to be created, and `eva.near` will wait for both `carol.near`
    /// and `dave.near` to be created first.
    ///
    /// ```no_run
    /// # use near_sdk::{Promise, VMContext, testing_env};
    /// let bob = "bob.near".parse().unwrap();
    /// let carol = "carol.near".parse().unwrap();
    /// let dave = "dave.near".parse().unwrap();
    /// let eva = "eva.near".parse().unwrap();
    ///
    /// Promise::new(&bob).create_account()
    ///     .then(Promise::new(&carol).create_account())
    ///     .and(Promise::new(&dave).create_account())
    ///     .then(Promise::new(&eva).create_account())
    ///     .schedule();
    /// ```
    pub fn then<O>(self, other: PromiseBuilder<O>) -> PromiseThen<O> {
        PromiseThen { after: self.schedule(), inner: other }
    }

    /// A specialized, relatively low-level API method. Allows to mark the given promise as the one
    /// that should be considered as a return value.
    ///
    /// In the below code `a1` and `a2` functions are equivalent.
    /// ```
    /// # use near_sdk::{ext_contract, Gas, near_bindgen, Promise};
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
    ///        contract_b::ext(&"bob_near".parse().unwrap()).b().schedule();
    ///     }
    ///
    ///     pub fn a2(&self) -> Promise {
    ///        contract_b::ext(&"bob_near".parse().unwrap()).b().schedule_as_return()
    ///     }
    /// }
    /// ```
    pub fn schedule_as_return(self) -> Promise<T> {
        let index = self.schedule();
        crate::env::promise_return(index);
        Promise { index, _marker: Default::default() }
    }

    // TODO docs
    pub fn schedule(self) -> PromiseIndex {
        let promise_index = crate::env::promise_batch_create(self.account_id);
        for action in self.actions {
            action.add(promise_index);
        }
        promise_index
    }
}

/// Until we implement strongly typed promises we serialize them as unit struct.
impl<T> BorshSchema for PromiseBuilder<'_, T>
where
    T: BorshSchema,
{
    fn add_definitions_recursively(
        definitions: &mut HashMap<borsh::schema::Declaration, borsh::schema::Definition>,
    ) {
        <T>::add_definitions_recursively(definitions);
    }

    fn declaration() -> borsh::schema::Declaration {
        <T>::declaration()
    }
}

// TODO docs
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FunctionCallOpts {
    pub deposit: Option<Balance>,
    pub static_gas: Option<Gas>,
    pub gas_weight: Option<GasWeight>,
}

// TODO docs
#[must_use]
#[derive(Debug)]
pub struct PromiseThen<'a, T = NoReturn> {
    after: PromiseIndex,
    inner: PromiseBuilder<'a, T>,
}

impl<T> PromiseThen<'_, T> {
    /// Schedules execution of another promise right after the current promise finish executing.
    pub fn then<O>(self, other: PromiseBuilder<O>) -> PromiseThen<O> {
        PromiseThen { after: self.schedule(), inner: other }
    }

    pub fn and<O>(self, other: impl SchedulablePromise<O>) -> PromiseAnd<T, O> {
        let mut promises = vec![self.schedule()];
        other.append_to_promises(&mut promises);
        PromiseAnd { promises, _marker: Default::default() }
    }

    // TODO docs
    pub fn schedule(self) -> PromiseIndex {
        let promise_index = crate::env::promise_batch_then(self.after, self.inner.account_id);
        for action in self.inner.actions {
            action.add(promise_index);
        }
        promise_index
    }

    // TODO docs
    pub fn schedule_as_return(self) -> Promise<T> {
        let index = self.schedule();
        crate::env::promise_return(index);
        Promise { index, _marker: Default::default() }
    }
}

#[must_use]
#[derive(Debug)]
pub struct PromiseAnd<L, R> {
    promises: Vec<PromiseIndex>,
    _marker: PhantomData<fn() -> (L, R)>,
}

impl<L, R> PromiseAnd<L, R> {
    pub fn and<O>(mut self, other: impl SchedulablePromise<O>) -> PromiseAnd<PromiseAnd<L, R>, O> {
        other.append_to_promises(&mut self.promises);
        PromiseAnd { promises: self.promises, _marker: Default::default() }
    }

    pub fn then<O>(self, other: PromiseBuilder<O>) -> PromiseThen<O> {
        PromiseThen { after: self.schedule(), inner: other }
    }

    pub fn schedule(self) -> PromiseIndex {
        crate::env::promise_and(&self.promises)
    }

    pub fn schedule_as_return(self) -> Promise<PromiseAnd<L, R>> {
        let index = self.schedule();
        crate::env::promise_return(index);
        Promise { index, _marker: Default::default() }
    }
}

impl<L, R> BorshSchema for PromiseAnd<L, R>
where
    L: BorshSchema,
    R: BorshSchema,
{
    fn add_definitions_recursively(
        definitions: &mut HashMap<borsh::schema::Declaration, borsh::schema::Definition>,
    ) {
        // TODO this might be able to recursively check the sub declarations, and if they are
        // `PromiseAnd`, then the tuple elements are pulled and flattened into one definition.
        // Currently, with nested `PromiseAnd`, the definition will look like (A, (B, (C, D)))
        // which is usable, but not as clear as it could be.
        Self::add_definition(
            Self::declaration(),
            borsh::schema::Definition::Tuple { elements: vec![L::declaration(), R::declaration()] },
            definitions,
        );
        <L>::add_definitions_recursively(definitions);
        <R>::add_definitions_recursively(definitions);
    }

    fn declaration() -> borsh::schema::Declaration {
        format!("PromiseAnd<{}, {}>", L::declaration(), R::declaration())
    }
}

pub struct Promise<T = NoReturn> {
    pub index: PromiseIndex,
    _marker: PhantomData<fn() -> T>,
}

impl Promise<NoReturn> {
    #[deprecated = "creating a promise directly is deprecated, use `PromiseBuilder::new` instead"]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(account_id: &AccountId) -> PromiseBuilder<'_, NoReturn> {
        PromiseBuilder::new(account_id)
    }
}

#[cfg(feature = "abi")]
impl<T> schemars::JsonSchema for Promise<T>
where
    T: schemars::JsonSchema,
{
    fn schema_name() -> String {
        <T as schemars::JsonSchema>::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <T as schemars::JsonSchema>::json_schema(gen)
    }
}

#[derive(Debug)]
enum PromiseAction<'a> {
    CreateAccount,
    DeployContract {
        code: &'a [u8],
    },
    FunctionCallWeight {
        function_name: &'a str,
        arguments: Vec<u8>,
        amount: Balance,
        gas: Gas,
        weight: GasWeight,
    },
    Transfer {
        amount: Balance,
    },
    Stake {
        amount: Balance,
        public_key: &'a PublicKey,
    },
    AddFullAccessKey {
        public_key: &'a PublicKey,
        nonce: u64,
    },
    AddAccessKey {
        public_key: &'a PublicKey,
        allowance: Balance,
        receiver_id: &'a AccountId,
        function_names: &'a str,
        nonce: u64,
    },
    DeleteKey {
        public_key: &'a PublicKey,
    },
    DeleteAccount {
        beneficiary_id: &'a AccountId,
    },
}

impl PromiseAction<'_> {
    fn add(&self, promise_index: PromiseIndex) {
        use PromiseAction::*;
        match self {
            CreateAccount => crate::env::promise_batch_action_create_account(promise_index),
            DeployContract { code } => {
                crate::env::promise_batch_action_deploy_contract(promise_index, code)
            }
            FunctionCallWeight { function_name, arguments, amount, gas, weight } => {
                crate::env::promise_batch_action_function_call_weight(
                    promise_index,
                    function_name,
                    arguments,
                    *amount,
                    *gas,
                    GasWeight(weight.0),
                )
            }
            Transfer { amount } => {
                crate::env::promise_batch_action_transfer(promise_index, *amount)
            }
            Stake { amount, public_key } => {
                crate::env::promise_batch_action_stake(promise_index, *amount, public_key)
            }
            AddFullAccessKey { public_key, nonce } => {
                crate::env::promise_batch_action_add_key_with_full_access(
                    promise_index,
                    public_key,
                    *nonce,
                )
            }
            AddAccessKey { public_key, allowance, receiver_id, function_names, nonce } => {
                crate::env::promise_batch_action_add_key_with_function_call(
                    promise_index,
                    public_key,
                    *nonce,
                    *allowance,
                    receiver_id,
                    function_names,
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

/// When the method can return either a promise or a value, it can be called with `PromiseOrValue::Promise`
/// or `PromiseOrValue::Value` to specify which one should be returned.
/// # Example
/// ```no_run
/// # use near_sdk::{ext_contract, near_bindgen, Gas, PromiseOrValue};
/// #[ext_contract]
/// pub trait ContractA {
///     fn a(&mut self) -> bool;
/// }
///
/// let value = Some(true);
/// let val: PromiseOrValue<bool> = if let Some(value) = value {
///     PromiseOrValue::Value(value)
/// } else {
///     contract_a::ext(&"bob_near".parse().unwrap()).a().schedule_as_return().into()
/// };
/// ```
pub enum PromiseOrValue<T> {
    Promise(Promise<T>),
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

impl<T> From<Promise<T>> for PromiseOrValue<T> {
    fn from(s: Promise<T>) -> Self {
        PromiseOrValue::Promise(s)
    }
}

impl<T> From<PromiseOrValue<PromiseOrValue<T>>> for PromiseOrValue<T> {
    fn from(s: PromiseOrValue<PromiseOrValue<T>>) -> Self {
        match s {
            PromiseOrValue::Promise(p) => {
                // Transmute the return type to the inner return type to avoid nested types.
                PromiseOrValue::Promise(Promise { index: p.index, _marker: Default::default() })
            }
            PromiseOrValue::Value(p @ PromiseOrValue::Promise(_)) => p,
            PromiseOrValue::Value(v @ PromiseOrValue::Value(_)) => v,
        }
    }
}

impl<T> From<Promise<PromiseOrValue<T>>> for PromiseOrValue<T> {
    fn from(s: Promise<PromiseOrValue<T>>) -> Self {
        PromiseOrValue::Promise(Promise::from(s))
    }
}

impl<T> From<Promise<PromiseOrValue<T>>> for Promise<T> {
    fn from(s: Promise<PromiseOrValue<T>>) -> Self {
        // Can ignore the fact that the result comes from a promise or direct return value.
        Promise { index: s.index, _marker: Default::default() }
    }
}

// TODO re-eval if we want this automatically
impl<T> From<PromiseThen<'_, T>> for PromiseOrValue<T> {
    fn from(promise: PromiseThen<'_, T>) -> Self {
        PromiseOrValue::Promise(promise.schedule_as_return())
    }
}

#[cfg(feature = "abi")]
impl<T: schemars::JsonSchema> schemars::JsonSchema for PromiseOrValue<T> {
    fn schema_name() -> String {
        <T as schemars::JsonSchema>::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        <T as schemars::JsonSchema>::json_schema(gen)
    }
}
