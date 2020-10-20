

pub mod actors {
macro_rules! actor {
    ($name:ident, $private_key_name:ident, $private_key:expr) => {
        pub const $private_key_name: [u8; 64] = hex!($private_key);
        lazy_static! {
            pub static ref $name: [u8; 32] = {
                let keypair: Keypair = Keypair::from_bytes(&$private_key_name).unwrap();
                keypair.public.to_bytes().try_into().unwrap()
            };
        }
    };
}
use std::convert::TryInto;
use ed25519_dalek::Keypair;
actor!(
    ALICE,
    ALICES_PRIVATE_KEY,
    "46cc11f8c694141a5bbc1026612d2c26a0e82033dcdcc973f7c3227832d3bce300b2388455446bd9a520db78771a954c9b04b7bb4c36b16c7326e59927275484"
);
actor!(
    BOB,
    BOBS_PRIVATE_KEY,
    "09ad6a4742f2fda1f9a79469cb1193ff2d25a9a91aac10870678067c2a7e8aed04e04f59dccb19b05eeb3aaeffa89aee8e6a378f8a92aefebad93f6c4737ea3c"
);
actor!(
    CAROL,
    CAROLS_PRIVATE_KEY,
    "90a9e95b755277c752d2d38ce26ba4d4a4aa1dc12f30f8838964cc0a0a9ad9bf08047976497e71ec1ecce11a11f40f5784532666a98503dfb58ba481f1fd71e4"
);
actor!(
    DAVE,
    DAVES_PRIVATE_KEY,
    "d0f4c713365b7deb11792a2fe4940f68880dbdecf8845f630f1bed7743e85c610c05784048181ba020740947f564481809303485ee83469cbb71cf7fa2f983bd"
);
actor!(
    ERIN,
    ERINS_PRIVATE_KEY,
    "cbd5ffe67f4c434cd21dcb3b647ff2e19c23340a86c615bbb02931283d508bde11120d8b6ef94d560d7ed8c3c8891d5fff164b69acc0382e0877002a51107b42"
);
actor!(
    EVE,
    EVES_PRIVATE_KEY,
    "132947dc65db2a108bd61c1c6c3c8bcedd47e24230eb8824527d7b4a8c989c9511511fa34332e9b119002f1d7305da28ed10ee3a52025eecbdd125bfc1e22906"
);
actor!(
    FRANK,
    FRANKS_PRIVATE_KEY,
    "37051675fb3454e5f89da696ccd935bb9281fb9583a668c1e362c8b4a5e969f3151030939fccd5c1a4e9c3cbf1cab5ec31606c7fdd3ee07394903622e6dc4421"
);
actor!(
    MALLORY,
    MALLORYS_PRIVATE_KEY,
    "7d87a30af66b14693c011ad03e9e76aaa56d683ecbe466ca56b1ae1c557c70003002f1a99a08d2c535928053987e88f31b597c0f641eefb1b885f630d4b81344"
);
}
