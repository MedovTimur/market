#![no_std]
use gstd::{collections::HashMap, exec, msg, prelude::*, ActorId};
use market_io::*;

struct Market {
    products: HashMap<String, ProductData>,
    purchases: HashMap<ActorId, Vec<PurchaseData>>,
    admin: ActorId,
    config: Config,
}

static mut MARKET: Option<Market> = None;

#[no_mangle]
extern "C" fn init() {
    let config = msg::load().expect("Unable to decode `Config`.");
    unsafe {
        MARKET = Some(Market {
            admin: msg::source(),
            config,
            products: HashMap::new(),
            purchases: HashMap::new(),
        })
    };
}

impl Market {
    fn add_product(
        &mut self,
        name: String,
        quantity: u128,
        price: u128,
    ) -> Result<MarketEvent, MarketError> {
        let msg_source = msg::source();
        if msg_source != self.admin {
            return Err(MarketError::NotAdmin);
        }
        if self.products.contains_key(&name) {
            return Err(MarketError::AlreadyExists);
        }
        if price < exec::env_vars().existential_deposit {
            return Err(MarketError::PriceLessThanExistentialDeposit);
        }

        let product_data = ProductData { quantity, price };
        self.products.insert(name.clone(), product_data);

        Ok(MarketEvent::ProductAdded {
            name,
            quantity,
            price,
        })
    }
    fn update_product_info(
        &mut self,
        name: String,
        quantity: Option<u128>,
        price: Option<u128>,
    ) -> Result<MarketEvent, MarketError> {
        let msg_source = msg::source();
        if msg_source != self.admin {
            return Err(MarketError::NotAdmin);
        }
        let product_data = self
            .products
            .get_mut(&name)
            .ok_or(MarketError::ThereIsNoSuchName)?;

        if let Some(quantity) = quantity {
            product_data.quantity = quantity;
        }
        if let Some(price) = price {
            product_data.price = price;
        }

        Ok(MarketEvent::ProductInfoUpdated {
            name,
            quantity,
            price,
        })
    }
    fn update_config(&mut self, config: Config) -> Result<MarketEvent, MarketError> {
        let msg_source = msg::source();
        if msg_source != self.admin {
            return Err(MarketError::NotAdmin);
        }
        self.config = config.clone();
        Ok(MarketEvent::ConfigUpdated { config })
    }
    fn delete_product(&mut self, name: String) -> Result<MarketEvent, MarketError> {
        let msg_source = msg::source();
        if msg_source != self.admin {
            return Err(MarketError::NotAdmin);
        }

        self.products
            .remove(&name)
            .ok_or(MarketError::ThereIsNoSuchName)?;

        Ok(MarketEvent::ProductDeleted { name })
    }
    fn buy(
        &mut self,
        msg_source: ActorId,
        msg_value: u128,
        name: String,
        quantity: u128,
        delivery_address: String,
    ) -> Result<MarketEvent, MarketError> {
        let Some(product_data) = self.products.get_mut(&name) else {
            return Err(MarketError::ThereIsNoSuchName);
        };
        if quantity == 0 {
            return Err(MarketError::ZeroQuantity);
        }
        if quantity > product_data.quantity {
            return Err(MarketError::QuantityExceeded);
        }

        let total_payment = product_data.price * quantity;
        if msg_value < total_payment {
            return Err(MarketError::InsufficientValue);
        } else if msg_value > total_payment {
            send_value(msg_source, msg_value - total_payment);
        }

        product_data.quantity -= quantity;

        let new_purchase = PurchaseData {
            name: name.clone(),
            quantity,
            status: Status::PaidFor,
            delivery_address,
        };
        self.purchases
            .entry(msg_source)
            .and_modify(|purchase| purchase.push(new_purchase.clone()))
            .or_insert(vec![new_purchase]);

        Ok(MarketEvent::Bought {
            buyer: msg_source,
            name,
            quantity,
        })
    }
}

fn send_value(destination: ActorId, value: u128) {
    if value != 0 {
        msg::send_with_gas(destination, "", 0, value).expect("Error in sending value");
    }
}

#[no_mangle]
extern "C" fn handle() {
    let action: MarketAction = msg::load().expect("Could not load `MarketAction`.");
    let market: &mut Market =
        unsafe { MARKET.as_mut().expect("Unexpected uninitialized `MARKET`.") };
    let result = match action {
        MarketAction::AddProduct {
            name,
            quantity,
            price,
        } => market.add_product(name, quantity, price),
        MarketAction::UpdateProductInfo {
            name,
            quantity,
            price,
        } => market.update_product_info(name, quantity, price),
        MarketAction::UpdateConfig { config } => market.update_config(config),
        MarketAction::DeleteProduct { name } => market.delete_product(name),
        MarketAction::Buy {
            name,
            quantity,
            delivery_address,
        } => {
            let msg_source = msg::source();
            let msg_value = msg::value();
            let result = market.buy(msg_source, msg_value, name, quantity, delivery_address);
            if result.is_err() {
                send_value(msg_source, msg_value);
            }
            result
        }
    };

    msg::reply(result, 0)
        .expect("Failed to encode or reply with `Result<MarketEvent, MarketError>`.");
}

#[no_mangle]
extern "C" fn state() {
    let market = unsafe { MARKET.take().expect("Unexpected error in taking state") };
    let query: StateQuery = msg::load().expect("Unable to load the state query");
    let reply = match query {
        StateQuery::All => StateReply::All(market.into()),
        StateQuery::GetProducts => StateReply::Products(market.products.into_iter().collect()),
        StateQuery::GetPurchases => StateReply::Purchases(market.purchases.into_iter().collect()),
        StateQuery::GetActorPurchases(actor_id) => {
            StateReply::ActorPurchases(market.purchases.get(&actor_id).or(None).cloned())
        }
    };
    msg::reply(reply, 0).expect("Unable to share the state");
}

impl From<Market> for State {
    fn from(value: Market) -> Self {
        let Market {
            products,
            purchases,
            admin,
            config,
        } = value;

        let products = products.into_iter().collect();
        let purchases = purchases.into_iter().collect();

        Self {
            products,
            purchases,
            admin,
            config,
        }
    }
}
