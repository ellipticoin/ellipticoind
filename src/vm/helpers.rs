use std::mem;
use std::mem::transmute;

pub fn i32_to_vec(n: i32) -> Vec<u8> {
    unsafe { transmute::<i32, [u8; mem::size_of::<i32>()]>(n) }.to_vec()
}

pub fn zero_pad_vec(vec: &[u8], len: usize) -> Vec<u8> {
    let mut padded = vec![0; len];
    padded[..vec.len()].clone_from_slice(vec);
    padded
}
