use crate::config::{my_public_key, my_signing_key};
use crate::system_contracts::ellipticoin::Miner;
use ellipticoin::{Address, BurnProofs, BurnTransaction, PublicKey, WitnessedMinerBlock};
use serde_cose::Sign1;
use std::collections::HashMap;

use crate::constants::MINERS;

#[derive(Clone, Debug)]
pub enum MinerBlockDecision {
    Burned(BurnProofs),
    Accepted(WitnessedMinerBlock),
}

#[derive(Clone, Debug)]
pub struct ExpectedBlock {
    pub number: u32,
    pub miner: Miner,
    pub decisions: HashMap<PublicKey, MinerBlockDecision>,
    pub burned_miners: HashMap<PublicKey, BurnProofs>,
}

// Note: None of the functions below are threadsafe. Lock to read / write.
impl ExpectedBlock {
    pub fn new(number: u32, miner: Miner) -> Self {
        Self {
            number,
            miner,
            burned_miners: HashMap::new(),
            decisions: HashMap::new(),
        }
    }

    pub fn increment(&mut self, miner: Miner) {
        self.number += 1;
        self.miner = miner;
        self.burned_miners.clear();
        self.decisions.clear();
    }

    pub fn is_miner_burned_by(&self, burned: &Miner, by: &Miner) -> bool {
        self.burned_miners.contains_key(&burned.address)
            && self
                .burned_miners
                .get(&burned.address)
                .unwrap()
                .contains_key(&by.address)
    }

    pub async fn is_miner_burned_by_me(&self, miner: &Miner) -> bool {
        self.is_miner_burned_by(miner, &(MINERS.me().await))
    }

    pub fn current_burn_proof(&self) -> Option<BurnProofs> {
        match self.burned_miners.get(&self.miner.address) {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }

    pub fn store_block_decision(&mut self, for_miner: &PublicKey, decision: &MinerBlockDecision) {
        if !self.decisions.contains_key(for_miner) {
            self.decisions.insert(for_miner.clone(), decision.clone());
        }
    }

    pub fn witness_miner_block(&mut self, block: &[u8]) -> WitnessedMinerBlock {
        let mut to_sign = Sign1::new(&block, my_public_key().to_vec());
        to_sign.sign(my_signing_key());
        self.decisions.insert(
            self.miner.address.clone(),
            MinerBlockDecision::Accepted(to_sign.clone()),
        );
        to_sign.clone()
    }

    pub fn burn_current_miner(
        &mut self,
        signed_burn_tx: &BurnTransaction,
        miner_count: usize,
        next_miner: &Miner,
    ) -> bool {
        let burn_count: usize;
        let burner_address: PublicKey = match Address::from(signed_burn_tx.kid()) {
            Address::PublicKey(pub_key) => pub_key,
            // TODO: Handle errors
            _ => return false,
        };

        match self.burned_miners.get_mut::<PublicKey>(&self.miner.address) {
            Some(map) => {
                map.insert(burner_address, signed_burn_tx.clone());
                burn_count = map.len();
            }
            None => {
                let mut inner = HashMap::new();
                inner.insert(self.miner.address.clone(), signed_burn_tx.clone());
                self.burned_miners.insert(burner_address.clone(), inner);
                burn_count = 1;
            }
        }

        // TODO: Eventually change from 100% to 51%
        if burn_count + 1 >= miner_count {
            self.miner = next_miner.clone();
            true
        } else {
            false
        }
    }
}
