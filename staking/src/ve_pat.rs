use crate::*;
use near_sdk::json_types:: {U128};
use near_sdk::{assert_one_yocto, env, log, ext_contract, AccountId, Gas, ONE_YOCTO, PromiseResult};

// Validator interface, for cross-contract calls
#[ext_contract(ft_contract)]
pub trait FungibleToken {
  fn ft_balance_of(&mut self, account_id: AccountId) -> U128;
  fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
  fn storage_deposit(
    &mut self,
    account_id: Option<AccountId>,
    registration_only: Option<bool>,
) -> StorageBalance;
}

#[ext_contract(voting_contract)]
pub trait Voting{
    fn create_poll(&mut self, question: String, variants: HashMap<String, String>, start: u64, end: u64, budget: U128) -> String;
    fn get_winner_voting(&mut self, poll_id: String) -> String;
}

#[ext_contract(ext_self)]
pub trait MyContract {
    fn end_voting_helper(&mut self, job_id: String, freelancer_id: AccountId);
}

pub const TGAS: u64 = 1_000_000_000_000;
pub const STORAGE_DEPOSIT_AMOUNT:u128 = 1_250_000_000_000_000_000_000;

#[near_bindgen]
impl Contract {

    pub fn transfer_owner(&mut self, new_owner_id: AccountId){
        let signer = env::signer_account_id();
        assert_eq!(signer, self.owner_id, "only owner");
        self.owner_id = new_owner_id.clone();
        log!("new owner_id is: {}", new_owner_id);
    }

    pub fn change_ft_contract_id(&mut self, new_ft_contract_id: AccountId){
        let signer = env::signer_account_id();
        assert_eq!(signer, self.owner_id, "only owner");
        self.ft_contract_id = new_ft_contract_id.clone();
        log!("new ft_contract_id is: {}", new_ft_contract_id);
    }

    pub fn change_voting_contract_id(&mut self, new_voting_contract_id: AccountId){
        let signer = env::signer_account_id();
        assert_eq!(signer, self.owner_id, "only owner");
        self.voting_contract_id = new_voting_contract_id.clone();
        log!("new_voting_contract_id is: {}", new_voting_contract_id);
    }

    /// Deposit PAT to mint vePAT tokens to the predecessor account in this contract.
    #[payable]
    pub fn deposit(&mut self, amount_stake: U128) {
        let amount = amount_stake.into();
        assert!(amount > 0, "amount must be > 0");       
        let signer_id = env::signer_account_id();

        if !self.ft.accounts.contains_key(&signer_id) {
            self.ft.internal_register_account(&signer_id);
        }
        self.ft.internal_deposit(&signer_id, amount);
        self.total_staked += amount;
        log!("Deposit {} PAT to {}", amount, signer_id);
    }

    /// Withdraws vePAT and send PAT back to the predecessor account. Requires attached deposit of exactly 1 yoctoNEAR.
    #[payable]
    pub fn withdraw(&mut self, amount_stake: U128) {
        assert_one_yocto();
        let account_id = env::signer_account_id();
        let amount = amount_stake.into();
        if !self.ft.accounts.contains_key(&account_id) {
            self.ft.internal_register_account(&account_id);
        }
        self.ft.internal_withdraw(&account_id, amount);
        self.total_staked -= amount;
        log!("Withdraw {} yoctoNEAR from {}", amount, account_id);

        ft_contract::ext(self.ft_contract_id.clone())
            .with_static_gas(Gas(TGAS))
            .with_attached_deposit(STORAGE_DEPOSIT_AMOUNT)
            .storage_deposit(Some(account_id.clone()), None).then(ft_contract::ext(self.ft_contract_id.clone())
                .with_static_gas(Gas(TGAS))
                .with_attached_deposit(ONE_YOCTO)
                .ft_transfer(account_id, amount_stake, None));
    }

    pub fn get_total_staked(&self) -> u128{
        return self.total_staked;
    }

    pub fn show_jobs(&self, jobs_id: String) -> Option<Jobs>{
        match self.jobs.get(&jobs_id){          
            Some(rs) => Some(Jobs { 
                creator_id: (rs.creator_id.clone()), 
                budget: (rs.budget.clone()), 
                freelancers: (rs.freelancers.clone()), 
                is_start: (rs.is_start.clone()), 
                is_end: (rs.is_end.clone()), 
                voting_id: (rs.voting_id.clone()) 
            }),
            None => None,
        }
    }

    pub fn get_list_jobs(&self) -> HashMap<String, Jobs>{
        return self.jobs.clone();
    }


    #[payable]
    pub fn create_jobs(&mut self, amount_stake: U128, para: String) -> String {
        let amount:u128 = amount_stake.into();
        assert!(amount > 0, "amount must be > 0");
        let signer_id = env::signer_account_id();
        let job_id = para;
        log!(format!("job_id is {}", job_id.clone()));
        self.jobs.insert(
            job_id.clone(),
            Jobs { creator_id: (signer_id), budget: (amount_stake.into()), freelancers: (HashMap::new()), is_start: (false), is_end: (false), voting_id: String::new()},
        );
        return job_id;
    }

    #[payable]
    pub fn get_jobs(&mut self, job_id: String) -> bool{
        assert_one_yocto();
        let signer_id = env::signer_account_id();
        match self.jobs.get_mut(&job_id){
            Some(results) =>{
                assert_eq!(results.is_start, false, "jobs be started");
                match results.freelancers.get_mut(&signer_id){
                    Some(result) => {
                        //when freelancer get_job , tuple (get, complete).0 is true 
                        result.freelancer = (true,false);
                    }
                    None => {
                        results.freelancers.insert(signer_id, Confirm { creator: (false,false), freelancer: (true,false) });
                    }
                }
                return true;
            }
            None => {
                log!(format!("no jobs known for {}", job_id));
                return false;
            }
        }
    }

    #[payable]
    pub fn start_jobs(&mut self, job_id: String, freelancer_id: AccountId) -> bool{
        assert_one_yocto();
        let signer_id = env::signer_account_id();
        match self.jobs.get_mut(&job_id){
            Some(results) =>{
                assert_eq!(results.creator_id, signer_id, "only creator");
                match results.freelancers.get_mut(&freelancer_id){
                    Some(result) => {
                        assert!(result.freelancer.0 == true, "freelancer not get jobs");
                        assert_eq!(results.is_start, false, "jobs be started");
                        result.creator = (true,false);
                        results.is_start = true;
                        return true;
                    }
                    None => {
                        return false;
                    }
                }
            }
            None => {
                log!(format!("no jobs known for {}", job_id));
                return false;
            }
        }
    }

    #[payable]
    pub fn complete_jobs(&mut self, job_id: String, choice: bool) -> bool{
        assert_one_yocto();
        let signer_id = env::signer_account_id();
        match self.jobs.get_mut(&job_id){
            Some(results) =>{
                match results.freelancers.get_mut(&signer_id){
                    Some(result) => {
                        assert_eq!(result.creator.0, true, "is not allow");
                        result.freelancer = (true,choice);
                        return true;
                    }
                    None => {
                        return false;
                    }
                }        
            }
            None => {
                log!(format!("no jobs known for {}", job_id));
                return false;
            }
        }
    }
    
    #[payable]
    pub fn end_jobs(&mut self, job_id: String, freelancer_id: AccountId) -> bool{
        assert_one_yocto();
        let signer_id = env::signer_account_id();
        match self.jobs.get_mut(&job_id){
            Some(results) =>{
                assert_eq!(results.is_end, false, "jobs be ended");
                assert_eq!(results.creator_id, signer_id, "only creator");
                match results.freelancers.get_mut(&freelancer_id){
                    Some(result) => {
                        assert_eq!(result.freelancer.1, true, "freelancer must be complete");
                        result.creator.1 = true;
                    }
                    None => {
                        return false;
                    }
                }

                // unlock token and send to freelancer
                let budget = results.budget;

                //transfer to freelancer
                ft_contract::ext(self.ft_contract_id.clone())
                    .with_static_gas(Gas(TGAS))
                    .with_attached_deposit(STORAGE_DEPOSIT_AMOUNT)
                    .storage_deposit(Some(freelancer_id.clone()), None).then(
                        ft_contract::ext(self.ft_contract_id.clone())
                        .with_static_gas(Gas(TGAS))
                        .with_attached_deposit(ONE_YOCTO)
                        .ft_transfer(freelancer_id.clone(), (budget*95/100).into(), None)
                    );

                // add point to freelancer_id
                match self.points.get_mut(&freelancer_id){
                    Some(result) =>{
                        *result += budget;
                    }
                    None => {
                        self.points.insert(freelancer_id, budget);
                    }
                }

                // add point to creator
                match self.points.get_mut(&results.creator_id){
                    Some(result) =>{
                        *result += budget; 
                    }
                    None => {
                        self.points.insert(results.creator_id.clone(), budget);
                    }
                }
                //transfer to protocol
                ft_contract::ext(self.ft_contract_id.clone())
                    .with_static_gas(Gas(TGAS))
                    .with_attached_deposit(STORAGE_DEPOSIT_AMOUNT)
                    .storage_deposit(Some(self.owner_id.clone()), None).then(
                        ft_contract::ext(self.ft_contract_id.clone())
                            .with_static_gas(Gas(TGAS))
                            .with_attached_deposit(ONE_YOCTO)
                            .ft_transfer(self.owner_id.clone(), (budget*5/100).into(), None));


                results.is_end = true;
                return true;
            }
            None => {
                log!(format!("no jobs known for {}", job_id));
                return false;
            }
        }
    }

    #[payable]
    pub fn create_voting(
            &mut self, 
            job_id: String, 
            freelancer_id: AccountId, 
            question: String, 
            variants: HashMap<String, String>, 
            start: u64, 
            end: u64) -> PromiseOrValue<String> {
        assert_one_yocto();
        let signer_id = env::signer_account_id();
        assert_eq!(self.owner_id, signer_id, "only owner");
        match self.jobs.get_mut(&job_id){
            Some(results) =>{
                let budget = (results.budget/10).into();
                match results.freelancers.get_mut(&freelancer_id){ 
                    Some(result) => {
                        assert_eq!(result.freelancer.0, true, "freelancer must be get jobs");
                        assert_eq!(result.creator.0, true, "creator must be get jobs");
                    }
                    None => {
                        panic!("not found freelancer_id: {}", freelancer_id)
                    }
                }
                let promise = voting_contract::ext(self.voting_contract_id.clone())
                    .with_static_gas(Gas(5_000_000_000_000))
                    .with_attached_deposit(0)
                    .create_poll(question, variants, start, end, budget);
                return PromiseOrValue::from(promise); 
            }
            None => {
                panic!("not found job_id: {}", job_id);
            
            }
        }
    }

    #[payable]
    pub fn end_voting(&mut self, job_id: String, freelancer_id: AccountId, poll_id: String){
        assert_one_yocto();
        let signer_id = env::signer_account_id();
        assert_eq!(self.owner_id, signer_id, "only owner");
        voting_contract::ext(self.voting_contract_id.clone())
            .with_static_gas(Gas(TGAS))
            .with_attached_deposit(0)
            .get_winner_voting(poll_id).then(ext_self::ext(env::current_account_id())
                .with_static_gas(Gas(TGAS))
                .with_attached_deposit(0)
                .end_voting_helper(job_id, freelancer_id));
    }

    pub fn end_voting_helper(&mut self, job_id: String, freelancer_id: AccountId){

        assert_eq!(env::promise_results_count(),1,"This is a callback method");
        match env::promise_result(0) {
            PromiseResult::NotReady => log!("NotReady".to_string()),
            PromiseResult::Failed => log!("Failed".to_string()),
            PromiseResult::Successful(rs) => {
                let winner = near_sdk::serde_json::from_slice::<String>(&rs).unwrap();
                
                match self.jobs.get_mut(&job_id){
                    Some(results) =>{
                        assert_eq!(results.is_end, false, "jobs be ended");
                        // unlock token and send to freelancer
                        let budget = results.budget;
        
                        let winner_id:AccountId;
                        match winner.as_str() {
                            "v1" => winner_id = results.creator_id.clone(),
                            "v2" => winner_id = freelancer_id,
                            _ => panic!("error winner_id"),
                        }
                        log!("winner_id is {}", winner_id);
                        //transfer to freelancer
                        ft_contract::ext(self.ft_contract_id.clone())
                            .with_static_gas(Gas(TGAS))
                            .with_attached_deposit(STORAGE_DEPOSIT_AMOUNT)
                            .storage_deposit(Some(winner_id.clone()), None).then(
                                ft_contract::ext(self.ft_contract_id.clone())
                                    .with_static_gas(Gas(TGAS))
                                    .with_attached_deposit(ONE_YOCTO)
                                    .ft_transfer(winner_id.clone(), (budget*9/10).into(), None));
                        match self.points.get_mut(&winner_id){
                            Some(result) =>{
                                *result += budget*1/2;
                            }
                            None => {
                                self.points.insert(winner_id, budget);
                            }
                        }
        
                        //transfer to voting
                        ft_contract::ext(self.ft_contract_id.clone())
                            .with_static_gas(Gas(TGAS))
                            .with_attached_deposit(STORAGE_DEPOSIT_AMOUNT)
                            .storage_deposit(Some(self.voting_contract_id.clone()), None).then(
                                ft_contract::ext(self.ft_contract_id.clone())
                                    .with_static_gas(Gas(TGAS))
                                    .with_attached_deposit(ONE_YOCTO)
                                    .ft_transfer(self.voting_contract_id.clone(), (budget*1/10).into(), None));
        
        
                        results.is_end = true;
                        
                    }
                    None => {
                        log!(format!("no jobs known for {}", job_id));
                        
                    }
                }

            },
        }
        
    }


    // pub fn test_transfer(&mut self, amount: U128) -> bool{
    //     let ft_contract_id = AccountId::new_unchecked("partie-test2.thanhdevtest.testnet".to_string());
    //     let signer = env::signer_account_id();
    //     //transfer to freelancer
    //     ft_contract::ext(ft_contract_id.clone())
    //         .with_static_gas(Gas(TGAS))
    //         .with_attached_deposit(STORAGE_DEPOSIT_AMOUNT)
    //         .storage_deposit(Some(signer.clone()), None).then(
    //             ft_contract::ext(ft_contract_id.clone())
    //             .with_static_gas(Gas(TGAS))
    //             .with_attached_deposit(ONE_YOCTO)
    //             .ft_transfer(signer.clone(), amount, None)
    //         );
    //     return true;
    // }   
}
