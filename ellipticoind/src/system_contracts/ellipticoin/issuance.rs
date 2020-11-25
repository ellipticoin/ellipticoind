use crate::system_contracts::token::{
    constants::{BTC, ETH},
    BASE_FACTOR,
};
use ellipticoin::Token;

lazy_static! {
    pub static ref INCENTIVISED_POOLS: Vec<Token> = vec![BTC.clone(), ETH.clone()];
}

const BLOCKS_PER_ERA: u32 = 8_000_000;
const NUMBER_OF_ERAS: u32 = 8;

const V1_ISSUANCE: u64 = 130_036_019_000;
const FIRST_ERA_BASE_ISSUANCE_PER_BLOCK: u64 = BASE_FACTOR * 128 / 100;
const LAST_BLOCK_OF_FIRST_ERA: u32 = (((BLOCKS_PER_ERA as u64 * FIRST_ERA_BASE_ISSUANCE_PER_BLOCK)
    - V1_ISSUANCE)
    / FIRST_ERA_BASE_ISSUANCE_PER_BLOCK) as u32;

pub fn block_reward_at(block: u32) -> u64 {
    if block > BLOCKS_PER_ERA * NUMBER_OF_ERAS {
        return 0;
    }
    if block <= LAST_BLOCK_OF_FIRST_ERA {
        return FIRST_ERA_BASE_ISSUANCE_PER_BLOCK as u64;
    }

    let era = ((block - LAST_BLOCK_OF_FIRST_ERA) / BLOCKS_PER_ERA) + 1;
    BASE_FACTOR * 128 * 10u64.pow(6) / 2u64.pow(era) / 10u64.pow(8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::BLOCK_TIME;
    use std::time::Duration;
    const NUMBER_OF_ERAS: u32 = 8;
    const SECONDS_IN_A_YEAR: u64 = 31556952;

    #[test]
    fn test_total_supply() {
        let mut total_issuance = 0;
        let mut total_time: Duration = Default::default();
        for era in 0..=NUMBER_OF_ERAS - 1 {
            let reward = block_reward_at(era * BLOCKS_PER_ERA);
            total_issuance += reward * BLOCKS_PER_ERA as u64;
            total_time += BLOCKS_PER_ERA * BLOCK_TIME.clone();
        }
        assert_eq!(
            block_reward_at((NUMBER_OF_ERAS as u32 * BLOCKS_PER_ERA) + 1),
            0
        );
        assert_eq!(total_issuance, 20400000 * BASE_FACTOR);
        assert_eq!(total_time.as_secs() / SECONDS_IN_A_YEAR, 6);
    }
}
