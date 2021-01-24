pub fn pad_left(value: Vec<u8>, padding_size: usize) -> Vec<u8> {
    let mut new_vec = vec![0; padding_size - value.len()];

    new_vec.splice(new_vec.len()..new_vec.len(), value.iter().cloned());
    new_vec
}

pub fn proportion_of(value: u64, x: u64, y: u64) -> u64 {
    (value as u128 * x as u128 / y as u128) as u64
}
