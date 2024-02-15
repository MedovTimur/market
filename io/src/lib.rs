#![no_std]

use gmeta::{In, InOut, Metadata};
use gstd::{prelude::*, ActorId};
pub struct MarketMetadata;

impl Metadata for MarketMetadata {
    type Init = In<Config>;
    type Handle = InOut<MarketAction, Result<MarketEvent, MarketError>>;
    type Others = ();
    type Reply = ();
    type Signal = ();
    type State = InOut<StateQuery, StateReply>;
}

#[derive(Debug, Encode, Decode, TypeInfo, Clone)]
pub struct Config {
    pub public_key: String,
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub struct ProductData {
    pub quantity: u128,
    pub price: u128,
}
#[derive(Debug, Encode, Decode, TypeInfo, Clone)]
pub struct PurchaseData {
    pub name: String,
    pub quantity: u128,
    pub status: Status,
    pub delivery_address: String,
}
#[derive(Debug, Encode, Decode, TypeInfo, Clone)]
pub enum Status {
    PaidFor,
    // TransmittedForDelivery and etc
}

#[derive(Encode, Decode, TypeInfo)]
pub enum MarketAction {
    AddProduct {
        name: String,
        quantity: u128,
        price: u128,
    },
    UpdateProductInfo {
        name: String,
        quantity: Option<u128>,
        price: Option<u128>,
    },
    UpdateConfig {
        config: Config,
    },
    DeleteProduct {
        name: String,
    },
    Buy {
        name: String,
        quantity: u128,
        delivery_address: String,
    },
}

#[derive(Encode, Decode, TypeInfo)]
pub enum MarketEvent {
    ProductAdded {
        name: String,
        quantity: u128,
        price: u128,
    },
    ProductInfoUpdated {
        name: String,
        quantity: Option<u128>,
        price: Option<u128>,
    },
    ConfigUpdated {
        config: Config,
    },
    ProductDeleted {
        name: String,
    },
    Bought {
        buyer: ActorId,
        name: String,
        quantity: u128,
    },
}

#[derive(Encode, Decode, TypeInfo)]
pub enum MarketError {
    NotAdmin,
    AlreadyExists,
    ThereIsNoSuchName,
    ZeroQuantity,
    PriceLessThanExistentialDeposit,
    InsufficientValue,
    QuantityExceeded,
}

#[derive(Encode, Decode, TypeInfo)]
pub enum StateQuery {
    All,
    GetProducts,
    GetPurchases,
    GetActorPurchases(ActorId),
}

#[derive(Encode, Decode, TypeInfo)]
pub enum StateReply {
    All(State),
    Products(Vec<(String, ProductData)>),
    Purchases(Vec<(ActorId, Vec<PurchaseData>)>),
    ActorPurchases(Option<Vec<PurchaseData>>),
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub struct State {
    pub products: Vec<(String, ProductData)>,
    pub purchases: Vec<(ActorId, Vec<PurchaseData>)>,
    pub admin: ActorId,
    pub config: Config,
}
