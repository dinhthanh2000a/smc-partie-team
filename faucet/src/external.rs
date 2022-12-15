use near_sdk::{ext_contract, json_types::U128, AccountId};

pub struct StorageBalance {
    pub total: U128,
    pub available: U128,
  }
// Interface for cross-contract FT calls
#[ext_contract(ft_contract)]
trait FtContract {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance;
}