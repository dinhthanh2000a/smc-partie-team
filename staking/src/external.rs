use near_sdk::{ext_contract, json_types::U128, AccountId};


// Validator interface, for cross-contract calls
#[ext_contract(ft_near)]
pub trait FungibleToken {
  fn ft_balance_of(&mut self, account_id: AccountId) -> U128;
  fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
  fn ft_resolve_transfer(&mut self, sender_id: AccountId, receiver_id: AccountId, amount: U128,) -> U128;
}
  
  