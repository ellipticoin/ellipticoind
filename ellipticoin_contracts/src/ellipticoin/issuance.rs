use super::Ellipticoin;
use crate::constants::{BASE_FACTOR, BTC, ETH};
use ellipticoin_types::Address;

lazy_static! {
    pub static ref INCENTIVISED_POOLS: Vec<Address> = vec![BTC.clone(), ETH.clone()];
}

const BLOCKS_PER_ERA: u64 = 8_000_000;
const NUMBER_OF_ERAS: u64 = 8;

impl Ellipticoin {
    pub fn block_reward_at(block: u64) -> u64 {
        if block > BLOCKS_PER_ERA * NUMBER_OF_ERAS {
            return 0;
        }
        let era = block / BLOCKS_PER_ERA;
        BASE_FACTOR * 128 * 10u64.pow(6) / 2u64.pow(era as u32) / 10u64.pow(8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::BLOCK_TIME;
    use std::time::Duration;
    const NUMBER_OF_ERAS: u64 = 8;
    const SECONDS_IN_A_YEAR: u64 = 31556952;

    #[test]
    fn test_total_supply() {
        let mut total_issuance = 0;
        let mut total_time: Duration = Default::default();
        for era in 0..=NUMBER_OF_ERAS - 1 {
            let reward = Ellipticoin::block_reward_at(era * BLOCKS_PER_ERA);
            total_issuance += reward * BLOCKS_PER_ERA as u64;
            total_time += (BLOCKS_PER_ERA as u32) * BLOCK_TIME.clone();
        }
        assert_eq!(
            Ellipticoin::block_reward_at((NUMBER_OF_ERAS * BLOCKS_PER_ERA) + 1),
            0
        );
        assert_eq!(total_issuance, 20400000 * BASE_FACTOR);
        assert_eq!(total_time.as_secs() / SECONDS_IN_A_YEAR, 8);
    }

    #[test]
    fn test_halvenings() {
        let mut last_block_reward: u64 = Ellipticoin::block_reward_at(BLOCKS_PER_ERA - 1);
        let mut block: u64 = BLOCKS_PER_ERA + 1;

        for _era in 1..=NUMBER_OF_ERAS - 1 {
            let new_reward: u64 = Ellipticoin::block_reward_at(block);
            assert_eq!(new_reward, last_block_reward / 2);
            block += BLOCKS_PER_ERA;
            last_block_reward = new_reward;
        }
    }
}
