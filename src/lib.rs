use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId};
use near_sdk::collections::{LookupSet, UnorderedSet, UnorderedMap};

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc = near_sdk::wee_alloc::WeeAlloc::INIT;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Asset {
    name: String,
    quantity: f64
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Transaction {
    sender : AccountId,
    receiver : AccountId,
    asset : Asset,
    satisfied : bool
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.sender == other.sender &&
        self.receiver == other.receiver &&
        self.asset.name == other.asset.name &&
        self.asset.quantity == other.asset.quantity &&
        self.satisfied == other.satisfied
    }
}

impl Transaction {

    pub fn new(sender: AccountId, receiver: AccountId, asset: Asset) -> Self {
        let satisfied = false;
        Self {
            sender,
            receiver,
            asset,
            satisfied
        }
    }

    pub fn toggle_satisfied(&mut self) {
        self.satisfied = !self.satisfied;
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    assets: UnorderedMap<AccountId, Asset>,
    transactions: UnorderedMap<String, Transaction>,
    owners: UnorderedSet<AccountId>,
    signatures : LookupSet<AccountId>,
}

impl Default for Contract {
    fn default() -> Self {
        env::panic("bonk".as_bytes())
    }
}

#[near_bindgen]
impl Contract {

    #[init]
    pub fn new(owners_in: Vec<AccountId>) -> Self {
        let mut owners = UnorderedSet::new(b"o".to_vec());
        for acct in owners_in.iter() {
            owners.insert(acct);
        }
        Self {
            assets: UnorderedMap::new(b"a".to_vec()),
            transactions: UnorderedMap::new(b"t".to_vec()),
            owners,
            signatures: LookupSet::new(b"s".to_vec()),
        }
    }

    pub fn add_tx(&mut self, tx_name: String, sender: AccountId, receiver: AccountId, asset_type: String, quantity: f64) {
        self.assert_owner();
        self.assert_no_agreement();
        let ass = Asset {
            name: asset_type,
            quantity: quantity
        };

        let tx = Transaction { 
            sender : sender,
            receiver : receiver,
            asset : ass,
            satisfied : false
        };
        self.transactions.insert(&tx_name, &tx);
    }

    pub fn rm_tx(&mut self, tx_name: String) {
        self.assert_owner();
        self.assert_no_agreement();
        self.transactions.remove(&tx_name);
    }

    pub fn get_tx(&self, tx_name: String)  -> Transaction {
        self.transactions.get(&tx_name).unwrap()
    }

    pub fn dep_asset(&mut self, asset: Asset, tx_name: String) {
        // TODO: cross contract to senders tokens to see if they got it
        self.assert_agreement();
        assert!(self.transactions.get(&tx_name).is_some(), "Transaction not in contract");
        let mut tx = self.transactions.get(&tx_name).unwrap();
        assert!(!&tx.satisfied, "Transaction has deposit already");
        assert_eq!(&tx.asset.name, &asset.name, "Asset being deposited does not match asset needed");
        assert_eq!(&tx.asset.quantity, &asset.quantity, 
            "{} needed, {} deposited", &tx.asset.quantity, &asset.quantity);
        let curr_user = env::current_account_id();
        assert_eq!(&tx.sender, &curr_user, "Asset needed from {}, not {}", &tx.sender, &curr_user);
        self.assets.insert(&curr_user, &asset);
        tx.toggle_satisfied();
    }

    // option available if all owners agree but one party does not deposit within reasonable time
    pub fn withdraw_asset(&mut self, tx_name: String) {
        // TODO: cross contract to senders tokens to see if they got it
        assert!(self.transactions.get(&tx_name).is_some(), "Transaction not in contract");
        let mut tx = self.transactions.get(&tx_name).unwrap();
        let curr_user = env::current_account_id();
        assert_eq!(&tx.sender, &curr_user, "Asset needed from {}, not {}", &tx.sender, &curr_user);
        self.assets.remove(&curr_user);
        // TODO: send to user
        tx.toggle_satisfied();
    }

    pub fn sign(&mut self) {
        self.assert_owner();
        let curr_user = env::current_account_id();
        self.signatures.insert(&curr_user); 
    }

    pub fn execute(&mut self) {
        self.assert_agreement();
        self.assert_txs_satisfied();
        for tx in self.transactions.iter() {
            // send asset to other contract
            println!("Giving {} {}s to {}", tx.1.asset.name, tx.1.asset.quantity, tx.1.receiver);
        }
        self.assets.clear();
    }
}

impl Contract {
    fn assert_owner(&self) {
        let curr_user = env::current_account_id();
        assert!(self.owners.contains(&curr_user), "only callable by owner");
    }

    fn assert_agreement(&self) {
        for owner in self.owners.iter() {
            assert!(self.signatures.contains(&owner), "Not all owners have agreed upon the terms");
        }
    }

    fn assert_no_agreement(&self) {
        for owner in self.owners.iter() {
            assert!(!self.signatures.contains(&owner), "Owners have already agreed upon the terms");
        }
    }

    fn assert_txs_satisfied(&self) {
        for tx in self.transactions.iter() {
            assert!(tx.1.satisfied, "Cannot execute transaction, {} must deposit {} {}s", 
                tx.1.sender, tx.1.asset.name, tx.1.asset.quantity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    fn bigpeepee69() -> String {
        "bigpeepee69".to_string()
    }

    fn bigpoopoo96() -> String {
        "bigpoopoo96".to_string()
    }


    fn get_context(input: Vec<u8>, is_view: bool, sender: AccountId) -> VMContext {
        VMContext {
            current_account_id: sender,
            signer_account_id: "bigpoopoo96.testnet".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "bigpoopoo96.testnet".to_string(),
            input,
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view,
            output_data_receivers: vec![],
            epoch_height: 19,
        }
    }

    #[test]
    fn add_tx() {
        // set up the mock context into the testing environment
        let context = get_context(vec![], false, bigpeepee69());
        testing_env!(context);
        let ass = Asset {
            name: "poop".to_string(),
            quantity: 4.0
        };
        let tx = Transaction::new(bigpeepee69(), 
            bigpoopoo96(), 
            ass);
        // instantiate a contract variable with the counter at zero
        let mut contract = Contract::new([bigpeepee69()].to_vec());
        contract.add_tx("shit trade".to_string(), bigpeepee69(), bigpoopoo96(), "poop".to_string(), 4.0);
        let same_tx = contract.get_tx("shit trade".to_string());
        assert!(same_tx == tx);
    }

    #[test]
    fn rm_tx() {
        // set up the mock context into the testing environment
        let context = get_context(vec![], false, bigpeepee69());
        testing_env!(context);
        let ass = Asset {
            name: "poop".to_string(),
            quantity: 4.0
        };
        let tx = Transaction::new(bigpeepee69(), 
            bigpoopoo96(), 
            ass);
        // instantiate a contract variable with the counter at zero
        let mut contract = Contract::new([bigpeepee69()].to_vec());
        contract.add_tx("shit trade".to_string(), bigpeepee69(), bigpoopoo96(), "poop".to_string(), 4.0);
        let same_tx = contract.get_tx("shit trade".to_string());
        assert!(same_tx == tx);
        contract.rm_tx( "shit trade".to_string());
        assert!(contract.transactions.get(&"shit trade".to_string()).is_none());
    }
}