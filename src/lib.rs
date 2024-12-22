#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Transaction {
    id: u64,
    phone_number: String,
    amount: u32,
    created_at: u64,
    status: String, // Example values: "Success", "Pending", "Failed"
}

// Implementasi trait Storable untuk Transaction
impl Storable for Transaction {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// Implementasi trait BoundedStorable untuk Transaction
impl BoundedStorable for Transaction {
    const MAX_SIZE: u32 = 512;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Transaction, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize)]
struct TransactionPayload {
    phone_number: String,
    amount: u32,
}

#[ic_cdk::update]
fn create_transaction(payload: TransactionPayload) -> Option<Transaction> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Cannot increment id counter");

    let transaction = Transaction {
        id,
        phone_number: payload.phone_number,
        amount: payload.amount,
        created_at: time(),
        status: "Pending".to_string(),
    };

    do_insert(&transaction);
    Some(transaction)
}

#[ic_cdk::query]
fn get_transaction(id: u64) -> Result<Transaction, Error> {
    match _get_transaction(&id) {
        Some(transaction) => Ok(transaction),
        None => Err(Error::NotFound {
            msg: format!("Transaction with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn update_transaction_status(id: u64, status: String) -> Result<Transaction, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut transaction) => {
            transaction.status = status;
            do_insert(&transaction);
            Ok(transaction)
        }
        None => Err(Error::NotFound {
            msg: format!("Cannot update transaction with id={}. Not found.", id),
        }),
    }
}

#[ic_cdk::update]
fn delete_transaction(id: u64) -> Result<Transaction, Error> {
    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(transaction) => Ok(transaction),
        None => Err(Error::NotFound {
            msg: format!("Cannot delete transaction with id={}. Not found.", id),
        }),
    }
}

// Helper function to insert a transaction into storage
fn do_insert(transaction: &Transaction) {
    STORAGE.with(|service| service.borrow_mut().insert(transaction.id, transaction.clone()));
}

// Helper function to get a transaction by id
fn _get_transaction(id: &u64) -> Option<Transaction> {
    STORAGE.with(|service| service.borrow().get(id))
}

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}

// Generate candid
ic_cdk::export_candid!();
