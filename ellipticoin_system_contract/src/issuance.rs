const YEARS_PER_ERA: u64 = 2;
const NUMBER_OF_ERAS: u64 = 7;
const SECONDS_PER_BLOCK: u64 = 5;
const LOG_BASE: u64 = 2;
const SCALE: u64 = 10000;
lazy_static! {
    static ref SECONDS_IN_A_YEAR: u64 = 60 * 60 * 24 * 365;
    static ref BLOCKS_PER_ERA: u64 = (YEARS_PER_ERA * *SECONDS_IN_A_YEAR)/ SECONDS_PER_BLOCK;
}

pub fn block_reward(block_number: u64) -> u64 {
    if era(block_number) < NUMBER_OF_ERAS {
        LOG_BASE.pow((NUMBER_OF_ERAS - era(block_number) - 1) as u32) * SCALE
    } else {
        0
    }
}

fn era(block_number: u64) -> u64 {
    block_number / *BLOCKS_PER_ERA
}

#[cfg(test)]
mod tests {
    use super::block_reward;

    #[test]
    fn test_block_reward() {
        for (block_number, reward) in vec![
            (0,640000),
            (1, 640000),
            (12614399, 640000),
            (12614400, 320000),
            (12614401, 320000),
            (25228800, 160000),
            (37843200, 80000),
            (50457600, 40000),
            (63072000, 20000),
            (75686400, 10000),
            (88300800, 0),
        ] {
            assert!(block_reward(block_number) == reward);
        }
    }
}
