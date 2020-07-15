#[macro_export]
macro_rules! impl_ellipticoin_externals {
    ($contract:ident) => {
        pub struct $contract<'a> {
            pub state: &'a mut State,
            pub transaction: Transaction,
            pub caller: Vec<u8>,
        }

        impl<'a> ellipticoin::MemoryAPI for $contract<'a> {
            fn get(&mut self, key: &[u8]) -> Vec<u8> {
                self.state
                    .get_memory(&self.transaction.contract_address, key)
            }

            fn set(&mut self, key: &[u8], value: &[u8]) {
                self.state
                    .set_memory(&self.transaction.contract_address, key, value)
            }
        }

        impl<'a> ellipticoin::StorageAPI for $contract<'a> {
            fn get(&mut self, key: &[u8]) -> Vec<u8> {
                self.state
                    .get_storage(&self.transaction.contract_address, key)
            }

            fn set(&mut self, key: &[u8], value: &[u8]) {
                self.state
                    .set_storage(&self.transaction.contract_address, key, value)
            }
        }

        impl<'a> ellipticoin::API for $contract<'a> {
            fn contract_address(&self) -> Vec<u8> {
                self.transaction.contract_address.clone()
            }
            fn sender(&self) -> Vec<u8> {
                self.transaction.sender.clone()
            }
            fn caller(&self) -> Vec<u8> {
                self.caller.clone()
            }
            fn call<
                D: DeserializeOwned
                    + 'static
                    + std::convert::From<ellipticoin::wasm_rpc::serde_cbor::Value>,
            >(
                _contract_address: Vec<u8>,
                _function_name: &str,
                _arguments: Vec<ellipticoin::wasm_rpc::serde_cbor::Value>,
            ) -> Result<D, ellipticoin::wasm_rpc::serde_cbor::Error> {
                ellipticoin::serde_cbor::from_slice(&vec![])
            }
        }
    };
}
