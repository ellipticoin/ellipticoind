pub trait ToKey {
    fn to_key(&self) -> Vec<u8>;
}

impl ToKey for u64 {
    fn to_key(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }
}

impl ToKey for [u8; 20] {
    fn to_key(&self) -> Vec<u8> {
        self.to_vec()
    }
}
