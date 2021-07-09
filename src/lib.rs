use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId};
use near_sdk::collections::{LookupMap, LookupSet, UnorderedSet, UnorderedMap};

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

impl Default for Transaction {
    fn default() -> Self {
        env::panic("Must set up Transaction properties properly".as_bytes())
    }
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
        let sender_signed = false;
        let receiver_signed = false;
        let satisfied = false;
        Self {
            sender,
            receiver,
            asset,
            satisfied
        }
    }

    pub fn set_satisfied(&mut self,value: bool) {
        self.satisfied = value
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
    assets: UnorderedMap<AccountId, Asset>,
    terms: UnorderedMap<String, Transaction>,
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
        let assets = UnorderedMap::new(b"a".to_vec());
        let terms = UnorderedMap::new(b"t".to_vec());
        let mut owners = UnorderedSet::new(b"o".to_vec());
        for acct in owners_in.iter() {
            owners.insert(acct);
        }
        let signatures = LookupSet::new(b"s".to_vec());
        Self {
            assets,
            terms,
            owners,
            signatures,
        }
    }

    pub fn add_tx(&mut self, owner: AccountId, tx_name: String, sender: AccountId, receiver: AccountId, asset_type: String, quantity: f64) {
        assert!(self.owners.contains(&owner), "you done fucked up now, only people included in contract can change it!");
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
        self.terms.insert(&tx_name, &tx);
    }

    pub fn rm_tx(&mut self, owner: AccountId, tx_name: String) {
        assert!(self.owners.contains(&owner), "you done fucked up now, only people included in contract can change it!");
        self.terms.remove(&tx_name);
    }

    pub fn get_tx(&self, tx_name: String)  -> Transaction {
        self.terms.get(&tx_name).unwrap()
    }

    pub fn dep_asset(&mut self, sender: AccountId, asset: Asset, tx_name: String) {
        // TODO: cross contract to senders tokens to see if they got it
        assert!(self.terms.get(&tx_name).is_some(), "Transaction not in contract");
        let mut tx = self.terms.get(&tx_name).unwrap();
        assert!(!&tx.satisfied, "Transaction has deposit already");
        assert_eq!(&tx.asset.name, &asset.name, 
            "Asset being deposited does not match asset needed");
        assert_eq!(&tx.asset.quantity, &asset.quantity, 
            "{} needed, {} deposited", &tx.asset.quantity, &asset.quantity);
        assert_eq!(&tx.sender, &sender, 
            "Asset needed from {}, not {}", &tx.sender, &sender);
        self.assets.insert(&sender, &asset);
        tx.set_satisfied(true);
    }

    pub fn withdraw_asset(&mut self, sender: AccountId, tx_name: String) {
        // TODO: cross contract to senders tokens to see if they got it
        assert!(self.terms.get(&tx_name).is_some(), "Transaction not in contract");
        let mut tx = self.terms.get(&tx_name).unwrap();
        assert_eq!(&tx.sender, &sender, 
            "Asset needed from {}, not {}", &tx.sender, &sender);
        self.assets.remove(&sender);
        // TODO: send to user
        tx.set_satisfied(false);
    }

    pub fn sign(&mut self, sender: AccountId) {
        assert!(self.owners.contains(&sender));
        self.signatures.insert(&sender); 
    }

    pub fn execute(&mut self) {
        // Assert that all owners have signed
        // iterate through owners, check to make sure each one's signature there, 
        for owner in self.owners.iter() {
            assert!(self.signatures.contains(&owner));
        }
        for (tx_name, tx) in self.terms.iter() {
            assert!(tx.satisfied, "Cannot execute transaction, {} must deposit {} {}s", 
                tx.sender, tx.asset.name, tx.asset.quantity);
        }
        for (tx_name, tx) in self.terms.iter() {
            // send asset to other contract
            println!("Giving {} {}s to {}", tx.asset.name, tx.asset.quantity, tx.receiver);
        }
        self.assets.clear();
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
            current_account_id: "bigpeepee69.testnet".to_string(),
            signer_account_id: "bigpoopoo96.testnet".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: sender,
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
        contract.add_tx(bigpeepee69(), "shit trade".to_string(), bigpeepee69(), bigpoopoo96(), "poop".to_string(), 4.0);
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
        contract.add_tx(bigpeepee69(), "shit trade".to_string(), bigpeepee69(), bigpoopoo96(), "poop".to_string(), 4.0);
        let same_tx = contract.get_tx("shit trade".to_string());
        assert!(same_tx == tx);
        contract.rm_tx(bigpeepee69(), "shit trade".to_string());
        assert!(contract.terms.get(&"shit trade".to_string()).is_none());
    }
}