#[derive(Debug)]
pub struct SledBackend {
    pub db: sled::Db,
}

// impl SledBackend {
//     pub fn new(path: String) -> Result<Self> {
//         Ok(Self {
//             db: sled::open(path)?,
//         })
//     }
// }

impl<'a> ellipticoin_types::db::Backend for SledBackend {
    fn get(&self, key: &[u8]) -> Vec<u8> {
        self.db
            .get(key)
            .unwrap_or(None)
            .map(|v| v.to_vec())
            .unwrap_or(vec![])
    }

    fn insert(&mut self, key: &[u8], value: &[u8]) {
        self.db.insert(key.to_vec(), value.to_vec()).unwrap();
    }
    // fn iter(&self) -> impl Iterator<Item=u8> + '_  { todo!() }
}

impl IntoIterator for SledBackend {
    type Item = (Vec<u8>, Vec<u8>);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.db.into_iter().map(|item| item.unwrap()).map(|(key, value)| (key.to_vec(), value.to_vec())).collect::<Vec<(Vec<u8>, Vec<u8>)>>().into_iter()
    }
}
