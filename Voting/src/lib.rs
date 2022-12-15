use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, ext_contract,log, near_bindgen, Gas, Balance, PanicOnDefault, PromiseResult, PromiseOrValue, AccountId, ONE_YOCTO};
use near_sdk::json_types::{U128};
use near_sdk::serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod external;
pub use crate::external::*;

pub const DECIMAL:u128 = 1_000_000_000_000_000_000;
pub const TGAS: u64 = 1_000_000_000_000;
pub const STORAGE_DEPOSIT_AMOUNT:u128 = 1_250_000_000_000_000_000_000;

#[ext_contract(ext_self)]
pub trait MyContract {
    fn my_callback(&self) -> String;
    fn vote_helper(&mut self, poll_id: String, votes: HashMap<String, i32>) -> bool;
}


#[derive(Serialize, Deserialize, Clone, BorshDeserialize, BorshSerialize)]
pub struct VotingOption {
    option_id: String,
    message: String,
}


#[derive(Serialize, Deserialize, Clone, BorshDeserialize, BorshSerialize)]
pub struct VotedDetail {
    option_id: String,
    quantity: u128,
    is_claimed: bool,
}

#[derive(Serialize, Deserialize, Clone, BorshDeserialize, BorshSerialize)]
pub struct VotingOptions {
    // Author of the vote (account id).
    creator: AccountId,
    // Unique voting id.
    poll_id: String,
    // Question voted on.
    question: String,
    variants: Vec<VotingOption>,
    // Start voting
    start: u64,
    // End voting
    end: u64,
    //budget
    budget: U128,
}

#[derive(Serialize, Deserialize, Clone, BorshDeserialize, BorshSerialize)]
pub struct VotingResults {
    // Unique poll id.
    poll_id: String,
    // Map of option id to the number of votes.
    variants: HashMap<String, u128>,
    // Map of voters who already voted.
    voted: HashMap<AccountId, VotedDetail>,
    // Total voted balance so far.
    total_voted_stake: Balance,
    
}

#[derive(Serialize, Deserialize)]
pub struct VotingStats {
    poll: VotingOptions,
    results: VotingResults,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Voting {
    owner_id: AccountId,
    // Map of poll id to voting options.
    polls: HashMap<String, VotingOptions>,
    // Map of poll id to voting results.
    results: HashMap<String, VotingResults>,
    
    ft_contract_id: AccountId,

    ve_ft_contract_id: AccountId,
}

#[near_bindgen]
impl Voting {


    #[init]
    pub fn new(owner_id: AccountId, ft_contract_id: AccountId, ve_ft_contract_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        let this = Self {
            owner_id: owner_id,
            polls: HashMap::new(),
            results: HashMap::new(),
            ft_contract_id: ft_contract_id,
            ve_ft_contract_id: ve_ft_contract_id
        };
        this
    }

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

    pub fn change_ve_ft_contract_id(&mut self, new_ve_ft_contract_id: AccountId){
        let signer = env::signer_account_id();
        assert_eq!(signer, self.owner_id, "only owner");
        self.ve_ft_contract_id = new_ve_ft_contract_id.clone();
        log!("new ft_contract_id is: {}", new_ve_ft_contract_id);
    }

    pub fn create_poll(&mut self, question: String, variants: HashMap<String, String>, start: u64, end: u64, budget: U128) -> String {
        assert!(start < end, "start must be less than end");
        log!(format!("create_poll for {} currently have {} polls",question,self.polls.len()));
        let creator_account_id = env::signer_account_id();
        let poll_id = bs58::encode(env::sha256(&env::random_seed())).into_string();
        let result = poll_id.clone();
        let mut variants_vec = <Vec<VotingOption>>::new();
        for (k, v) in variants.iter() {
            variants_vec.push(VotingOption {
                option_id: k.to_string(),
                message: v.to_string(),
            })
        }
        self.polls.insert(
            poll_id.clone(),
            VotingOptions {
                creator: creator_account_id,
                poll_id: poll_id.clone(),
                question: question,
                variants: variants_vec,
                start: start,
                end: end,
                budget: budget,
            },
        );
        self.results.insert(
            poll_id.clone(),
            VotingResults {
                poll_id: poll_id,
                variants: HashMap::new(),
                voted: HashMap::new(),
                total_voted_stake: 0,
                
            },
        );
        return result;
    }

    pub fn show_poll(&self, poll_id: String) -> Option<VotingOptions> {
        match self.polls.get(&poll_id) {
            Some(options) => Some(options.clone()),
            None => {
                log!(format!("Unknown voting {}", poll_id));
                None
            }
        }
    }

    pub fn show_results(&self, poll_id: String) -> Option<VotingStats> {
        match self.polls.get(&poll_id) {
            Some(poll) => match self.results.get(&poll_id) {
                Some(results) => Some(VotingStats {
                    results: results.clone(),
                    poll: poll.clone(),
                }),
                None => None,
            },
            None => None,
        }
    }

    pub fn show_list_voting(&self) -> HashMap<String, VotingOptions> {
        return self.polls.clone();
    }

    pub fn end_poll(&mut self, poll_id: String) -> Option<VotingStats> {
        match self.polls.get(&poll_id) {
            Some(poll) => {
                let block_ts = u64::from(env::block_timestamp());
                assert!(block_ts > poll.end, "must be poll ended");
                match self.results.get(&poll_id) {
                    
                    Some(results) => Some(VotingStats {
                        results: results.clone(),
                        poll: poll.clone(),
                    }),
                    None => None,
                }
            },
            None => None,
        }
    }

    pub fn get_winner_voting(&mut self, poll_id: String) -> String {
        match self.polls.get(&poll_id) {
            Some(poll) => {
                let block_ts = u64::from(env::block_timestamp());
                assert!(block_ts > poll.end, "must be poll ended");
                match self.results.get(&poll_id) {
                    
                    Some(results) => {
                        let mut v1: (String, u128) = ("v1".to_string(), 0);
                        let mut v2: (String, u128) = ("v2".to_string(), 0);
                        match results.variants.get(&"v1".to_string()) {
                            Some(rs) => v1 = ("v1".to_string(), *rs),
                            None => log!("v1 not found"),
                        }
                        match results.variants.get(&"v2".to_string()) {
                            Some(rs) => v2 = ("v2".to_string(), *rs),
                            None => log!("v2 not found"),
                        }
                        let eq = v1.1 > v2.1;
                        match eq {
                            true => return v1.0,
                            false => return v2.0,
                        }
                    },
                    None => return "None".to_string(),
                }
            },
            None => return "None".to_string(),
        }
    }

    #[payable]
    pub fn claim_reward(&mut self, poll_id: String) -> bool {
        let signer = env::signer_account_id();
        // let total_staked:u128 = match self.results.get(&poll_id) {
        //     Some(rs) =>{
        //         rs.total_voted_stake
        //     },
        //     None => 0,
        // };
        // log!("total_staked: {}",total_staked);

        let winner = self.get_winner_voting(poll_id.clone());
        match self.polls.get_mut(&poll_id) {
            Some(poll) => {
                let block_ts = u64::from(env::block_timestamp());
                assert!(block_ts > poll.end, "must be poll ended");
                let budget:u128 = poll.budget.into();
                log!("budget: {}",budget);
                match self.results.get_mut(&poll_id) {
                    
                    Some(results) => {                       
                        match results.voted.get_mut(&signer) {
                            Some(rs) => {
                                assert_eq!(winner, rs.option_id, "you voted wrong");
                                assert_eq!(rs.is_claimed, false, "claimed!");
                                let total_voted = match results.variants.get(&winner.clone()){
                                    Some(value) => *value,
                                    None => 0,
                                };
                                log!("total_voted: {}",total_voted);
                                log!("quantity: {}", rs.quantity);
                                let claim_amount = budget*(rs.quantity/total_voted);
                                log!("claim_amount: {}", claim_amount);
                                ft_near::ext(self.ft_contract_id.clone())
                                    .with_static_gas(Gas(TGAS))
                                    .with_attached_deposit(STORAGE_DEPOSIT_AMOUNT)
                                    .storage_deposit(Some(signer.clone()), None).then(
                                        ft_near::ext(self.ft_contract_id.clone())
                                            .with_static_gas(Gas(TGAS))
                                            .with_attached_deposit(ONE_YOCTO)
                                            .ft_transfer(signer.clone(), claim_amount.into(), None));
                                rs.is_claimed = true;
                                log!(format!("{} claimed {} PAT", signer, claim_amount));
                                return true
                            },
                            None => return false,
                        }
                    },
                    None => return false,
                }
            },
            None => return false,
        }
    }      

    pub fn update_time_voting(&mut self, poll_id: String, start: u64, end: u64) -> bool{
        let signer = env::signer_account_id();
        
        assert!(start < end, "start must be less than end");
        match self.polls.get_mut(&poll_id){
            Some(results) =>{
                assert_eq!(signer, results.creator, "must be creator");
                results.start = start;
                results.end = end;
                return true;
            }
            None => {
                log!(format!("no poll known for {}", poll_id));
                return false;
            }
        }
    }

    pub fn ping(&mut self) -> String {
        // assert!(self.end.is_none(), "Voting has already ended");
        // let cur_epoch_height = env::epoch_height();
        // if cur_epoch_height != self.last_epoch_height{
        //     self.last_epoch_height = cur_epoch_height;
        // }                
        "PONG".to_string()
    }

    pub fn vote(&self, poll_id: String, votes: HashMap<String, i32>) -> PromiseOrValue<bool> {
        let signer = env::signer_account_id();
        let promise = ft_near::ext(self.ve_ft_contract_id.clone())
            .with_static_gas(Gas(5_000_000_000_000))
            .with_attached_deposit(0)
            .ft_balance_of(signer).then(ext_self::ext(env::current_account_id())
                .with_static_gas(Gas(5_000_000_000_000))
                .with_attached_deposit(0)
                .vote_helper(poll_id, votes)
        );
        return PromiseOrValue::from(promise);            
    }    

    pub fn vote_helper(&mut self, poll_id: String, votes: HashMap<String, i32>) -> bool {
        assert_eq!(env::promise_results_count(),1,"This is a callback method");
        let voter_contract = env::signer_account_id();
        let owner_contract = env::current_account_id();
        match env::promise_result(0) {
            PromiseResult::NotReady => return false,
            PromiseResult::Failed => return false,
            PromiseResult::Successful(result) => {
                let balance = near_sdk::serde_json::from_slice::<U128>(&result).unwrap();
                let staked_token = balance.0 ;
                log!(format!("staked_token: {}", staked_token));
                assert!(staked_token > 0, "{} is not a validator", voter_contract);
                log!(format!("{} is voting on {} owner is {} with {} tokens",voter_contract, poll_id, owner_contract, staked_token));
                match self.polls.get_mut(&poll_id){
                    Some(results) =>{
                        let block_ts = u64::from(env::block_timestamp());
                        let eq = results.start <=  block_ts;
                        assert!(eq, "voting not begin with start:{} and block_ts:{}", results.start , block_ts);
                        let eq = results.end >= block_ts ;
                        assert!(eq, "voting be ended with end:{} and block_ts:{}", results.end , block_ts);
                    }
                    None => {
                        log!(format!("no poll known for {}", poll_id));
                        return false;
                    }
                }
                // Now we need to find a contract to vote for.
                match self.results.get_mut(&poll_id) {
                    Some(results) => { 
                        match results.voted.get(&voter_contract) {
                            Some(_) => {
                                log!( format!("{} already voted in {}", voter_contract, poll_id));
                                return false;
                            }
                            None => {
                                for (vote, checked) in votes.iter() {
                                    if *checked == 0 {
                                        continue;
                                    }
                                    else{
                                        results.voted.insert(voter_contract.clone(), VotedDetail { 
                                            option_id: (vote.to_string()),  
                                            quantity: (staked_token),
                                            is_claimed: false,
                                        }); 
                                    }
                                };                                                             
                            }
                        }
                        for (vote, checked) in votes.iter() {
                            if *checked == 0 {
                                continue;
                            }
                            match results.variants.get_mut(vote) {
                                Some(result) => {
                                    *result = *result + staked_token;
                                }
                                None => {
                                    results.variants.insert(vote.to_string(), staked_token);
                                }
                            }
                        }
                        results.total_voted_stake = results.total_voted_stake + staked_token ;
                        return true;
                    }
                    None => {
                        log!(format!("no poll known for {}", poll_id));
                        return false;
                    }
                };
            },
        };    
    }
    
}
