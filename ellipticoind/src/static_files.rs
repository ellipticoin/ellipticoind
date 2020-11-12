use std::convert::TryInto;
lazy_static! {
    pub static ref STATIC_FILES: [(&'static str, [u8; 32]); 3] = [
        (
            "ethereum-balances-10054080.bin",
            hex::decode("05f65b7b495ae773e928b6804921c4bd06441b10cceb914b618f5583663df5fa")
                .unwrap()[..]
                .try_into()
                .unwrap()
        ),
        (
            "genesis.cbor",
            hex::decode("cbf0a5a3b26b1a00327386ba04616fc0e412ae8eaf60ed91c1dacce586b58758")
                .unwrap()[..]
                .try_into()
                .unwrap()
        ),
        (
            "genesis-blocks.cbor",
            hex::decode("d01de8d49a4207f2fa0382e11302959ebbe00baf2b7ae075af5afbd263922f0f")
                .unwrap()[..]
                .try_into()
                .unwrap()
        )
    ];
}
