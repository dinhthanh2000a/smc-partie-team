use near_sdk::{
    assert_self,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::LookupSet,
    assert_one_yocto,
    env,
    json_types::U128,
    near_bindgen, require, Gas, AccountId, Balance, PromiseOrValue, ONE_YOCTO,
};
use std::collections::HashMap;

pub mod external;
pub use crate::external::*;

// 1 hour in MS
pub const REQUEST_GAP_LIMITER: u64 = 3_600_000;
pub const TGAS: u64 = 1_000_000_000_000;
pub const STORAGE_DEPOSIT_AMOUNT:u128 = 1_250_000_000_000_000_000_000;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    recent_contributions: Vec<(AccountId, Balance)>,
    recent_receivers: HashMap<AccountId, u64>,
    blacklist: LookupSet<AccountId>,
    ft_request_allowance: u128,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            recent_contributions: Vec::new(),
            recent_receivers: HashMap::new(),
            blacklist: LookupSet::new(b"s"),
            ft_request_allowance: 10_000_000_000_000_000_000_000,
        }
    }
}


#[near_bindgen]
impl Contract {

    #[private]
    pub fn admin_faucet(
        &mut self,
        ft_contract_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    )-> PromiseOrValue<U128> {
        let promise = ft_contract::ext(ft_contract_id)
            .with_static_gas(Gas(TGAS))
            .with_attached_deposit(ONE_YOCTO)
            .ft_transfer(receiver_id, amount, None);
        return PromiseOrValue::from(promise);
    }

    #[payable]
    pub fn ft_request_funds(
        &mut self,
        ft_contract_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> PromiseOrValue<U128> {
        
        assert_one_yocto();
        require!(
            self.blacklist.contains(&env::predecessor_account_id()) == false,
            "Account has been blacklisted!"
        );

        require!(
            amount.0 <= self.ft_request_allowance,
            "Requested amount is higher than the allowance"
        );
        let current_timestamp_ms: u64 = env::block_timestamp_ms();
        self.recent_receivers
            .retain(|_, v: &mut u64| *v + REQUEST_GAP_LIMITER > current_timestamp_ms);

        match self.recent_receivers.get(&receiver_id) {
            Some(previous_timestamp_ms) => {
                // if they did receive within the last ~30 min block them
                if &current_timestamp_ms - previous_timestamp_ms < REQUEST_GAP_LIMITER {
                    env::panic_str(
                        "You have to wait for a little longer before requesting to this account!",
                    )
                }
            }
            None => {
                self.recent_receivers
                    .insert(receiver_id.clone(), current_timestamp_ms);
            }
        }
        // require!(
        //     amount.0 <= ft_contract.ft_available_balance,
        //     "Requested amount is higher than the available balance of",
        // );
        
        // TODO Check/Pay the user storage_deposit
        /* TODO Check for recent_receivers
            this would require design decisions on how to handle multiple requests be in in the main recent receivers or
            in a separate list of recent receivers for each token
        */

        // Conditions are met we can transfer the funds
        let promise = ft_contract::ext(ft_contract_id.clone())
                    .with_static_gas(Gas(TGAS))
                    .with_attached_deposit(STORAGE_DEPOSIT_AMOUNT)
                    .storage_deposit(Some(receiver_id.clone()), None).then(
                        ft_contract::ext(ft_contract_id.clone())
                        .with_static_gas(Gas(TGAS))
                        .with_attached_deposit(ONE_YOCTO)
                        .ft_transfer(receiver_id.clone(), amount, None)
                    );
        // let promise = ft_contract::ext(ft_contract_id)
        //     .with_static_gas(Gas(TGAS))
        //     .with_attached_deposit(ONE_YOCTO)
        //     .ft_transfer(receiver_id, amount, None);
        return PromiseOrValue::from(promise);
            
    }

    // #[private] this macro does not expand for unit testing therefore I'm ignoring it for the time being
    pub fn add_to_blacklist(&mut self, account_id: AccountId) {
        assert_self();
        self.blacklist.insert(&account_id);
    }

    pub fn batch_add_to_blacklist(&mut self, accounts: Vec<AccountId>) {
        assert_self();
        // sadly no append TODO: Optimise
        for account in accounts {
            self.blacklist.insert(&account);
        }
    }

    // #[private] this macro does not expand for unit testing therefore I'm ignoring it for the time being
    pub fn remove_from_blacklist(&mut self, account_id: AccountId) {
        assert_self();
        self.blacklist.remove(&account_id);
    }

    // #[private] this macro does not expand for unit testing therefore I'm ignoring it for the time being
    pub fn clear_recent_receivers(&mut self) {
        assert_self();
        self.recent_receivers.clear();
    }


}