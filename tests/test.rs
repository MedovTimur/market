use gstd::{Encode};
use gtest::{Program, System};
use market_io::*;

pub const USERS: [u64; 3] = [10, 11, 12];
pub const ADMIN: u64 = 100;

pub trait TestFunc {
    fn add_product(&self, from: u64, name: String, quantity: u128, price: u128, error: Option<MarketError>);
    fn update_product_info(&self, from: u64, name: String, quantity: Option<u128>, price: Option<u128>, error: Option<MarketError>);
    fn delete_product(&self, from: u64, name: String, error: Option<MarketError>);
    fn buy(&self, from: u64, value: u128, name: String, quantity: u128, delivery_address: String, error: Option<MarketError>);
    fn get_all_state(&self) -> Option<State>;
}

impl TestFunc for Program<'_> {
    fn add_product(&self, from: u64, name: String, quantity: u128, price: u128, error: Option<MarketError>) {
        let result = self.send(from, MarketAction::AddProduct { name: name.clone(), quantity, price});
        assert!(!result.main_failed());
        let reply = if let Some(error) = error {
            Err(error)
        } else {
            Ok(MarketEvent::ProductAdded {
                name,
                quantity,
                price,
            })
        };
        assert!(result.contains(&(from, reply.encode())));
    }
    fn update_product_info(&self, from: u64, name: String, quantity: Option<u128>, price: Option<u128>, error: Option<MarketError>) {
        let result = self.send(from, MarketAction::UpdateProductInfo { name: name.clone(), quantity, price });
        assert!(!result.main_failed());
        let reply = if let Some(error) = error {
            Err(error)
        } else {
            Ok(MarketEvent::ProductInfoUpdated { name, quantity, price })
        };
        assert!(result.contains(&(from, reply.encode())));
    }
    fn delete_product(&self, from: u64, name: String, error: Option<MarketError>) {
        let result = self.send(from, MarketAction::DeleteProduct { name: name.clone() } );
        assert!(!result.main_failed());
        let reply = if let Some(error) = error {
            Err(error)
        } else {
            Ok(MarketEvent::ProductDeleted { name })
        };
        assert!(result.contains(&(from, reply.encode())));
    }
    fn buy(&self, from: u64, value: u128, name: String, quantity: u128, delivery_address: String, error: Option<MarketError>) {
        let result = self.send_with_value(from, MarketAction::Buy { name: name.clone(), quantity, delivery_address }, value);
        assert!(!result.main_failed());
        let reply = if let Some(error) = error {
            Err(error)
        } else {
            Ok(MarketEvent::Bought { buyer: from.into(), name: name.clone(), quantity })
        };
        assert!(result.contains(&(from, reply.encode())));
    }
    fn get_all_state(&self) -> Option<State> {
        let reply = self
            .read_state(StateQuery::All)
            .expect("Unexpected invalid state.");
        if let StateReply::All(state) = reply {
            Some(state)
        } else {
            None
        }
    }

}


#[test]
fn success_add_update_buy_delete_product() {
    let system = System::new();
    system.init_logger();
    let market = Program::current_opt(&system);
    let config = Config {
        public_key: "public key".to_string(),
    };
    let result = market.send(ADMIN, config);
    assert!(!result.main_failed());

    // Add Product
    let price = 10_000_000_000_000;
    let quantity = 100;
    market.add_product(ADMIN, "Product_#1".to_string(), quantity, price, None);

    let state: State = market.get_all_state().expect("Unexpected invalid game state.");
    assert_eq!(state.products.len(), 1);

    // Buy Product
    system.mint_to(USERS[0], 2*price);
    market.buy(USERS[0], 2*price, "Product_#1".to_string(), 1, "delivery_address".to_string(), None);
    // check to make sure the change is returned.
    system.claim_value_from_mailbox(USERS[0]);
    let balance = system.balance_of(USERS[0]);
    assert_eq!(balance, price);

    let state: State = market.get_all_state().expect("Unexpected invalid game state.");
    assert_eq!(state.products[0].1.quantity, quantity-1);
    assert_eq!(state.purchases.len(), 1);

    // Update Info Product
    market.update_product_info(ADMIN, "Product_#1".to_string(), Some(quantity*2), Some(price*2), None);
    let state: State = market.get_all_state().expect("Unexpected invalid game state.");
    assert_eq!(state.products[0].1.quantity, quantity*2);
    assert_eq!(state.products[0].1.price, price*2);

    // Buy Product witn new info of product
    // the user currently has `price` left on his balance, let's add more `price`
    system.mint_to(USERS[0], price);
    market.buy(USERS[0], 2*price, "Product_#1".to_string(), 1, "delivery_address".to_string(), None);

    system.claim_value_from_mailbox(USERS[0]);
    let balance = system.balance_of(USERS[0]);
    assert_eq!(balance, 0);

    let state: State = market.get_all_state().expect("Unexpected invalid game state.");
    assert_eq!(state.products[0].1.quantity, quantity*2-1);
    assert_eq!(state.purchases[0].1.len(), 2);

    // Delete product

    market.delete_product(ADMIN, "Product_#1".to_string(), None);
    let state: State = market.get_all_state().expect("Unexpected invalid game state.");
    assert_eq!(state.products.len(), 0);

}
#[test]
fn failures_add_product() {
    let system = System::new();
    system.init_logger();
    let market = Program::current_opt(&system);
    let config = Config {
        public_key: "public key".to_string(),
    };
    let result = market.send(ADMIN, config);
    assert!(!result.main_failed());

    let price = 10_000_000_000_000;
    market.add_product(USERS[0], "Product_#1".to_string(), 100, price, Some(MarketError::NotAdmin));
    market.add_product(ADMIN, "Product_#1".to_string(), 100, price-1, Some(MarketError::PriceLessThanExistentialDeposit));
    market.add_product(ADMIN, "Product_#1".to_string(), 100, price, None);
    market.add_product(ADMIN, "Product_#1".to_string(), 100, price, Some(MarketError::AlreadyExists));
}

#[test]
fn failures_bought() {
    let system = System::new();
    system.init_logger();
    let market = Program::current_opt(&system);
    let config = Config {
        public_key: "public key".to_string(),
    };
    let result = market.send(ADMIN, config);
    assert!(!result.main_failed());

    let price = 10_000_000_000_000;
    market.add_product(ADMIN, "Product_#1".to_string(), 100, price, None);

    system.mint_to(USERS[0], price);
    market.buy(USERS[0], price, "Product_#2".to_string(), 1, "delivery_address".to_string(), Some(MarketError::ThereIsNoSuchName));
    system.claim_value_from_mailbox(USERS[0]);
    market.buy(USERS[0], price, "Product_#1".to_string(), 0, "delivery_address".to_string(), Some(MarketError::ZeroQuantity));
    system.claim_value_from_mailbox(USERS[0]);
    market.buy(USERS[0], price, "Product_#1".to_string(), 101, "delivery_address".to_string(), Some(MarketError::QuantityExceeded));
    system.claim_value_from_mailbox(USERS[0]);
    market.buy(USERS[0], price, "Product_#1".to_string(), 2, "delivery_address".to_string(), Some(MarketError::InsufficientValue));


}
