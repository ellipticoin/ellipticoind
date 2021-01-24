use anyhow::{anyhow, bail, Result};
use core::array::TryFromSliceError;
use ellipticoin_contracts::{
    constants::{BASE_FACTOR, BTC, ELC, ETH, USD},
    Action, Transaction,
};
use ellipticoin_types::{Address, ADDRESS_LENGTH, DB};
use k256::{
    ecdsa::{recoverable, VerifyingKey},
    elliptic_curve::sec1::ToEncodedPoint,
};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::convert::{TryFrom, TryInto};

pub trait Signed: core::fmt::Debug {
    fn sender(&self) -> Result<[u8; 20]>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedTransaction(pub Transaction<Action>, Vec<u8>);
impl SignedTransaction {
    pub async fn run<D: DB>(&self, db: &mut D) -> Result<()> {
        self.0.run(db, self.sender()?)
    }
}

const SIGNATURE_LENGTH: usize = 65;

impl Signed for SignedTransaction {
    fn sender(&self) -> Result<[u8; 20]> {
        recover_signed_message(&self.0.verification_string()?, &self.1)
    }
}

pub trait VerificationString {
    fn verification_string(&self) -> Result<String>;
}

impl VerificationString for Action {
    fn verification_string(&self) -> Result<String> {
        match &self {
            Action::CreatePool(amount, token, intial_price) => Ok(format!(
                "Create a pool of {} {} at an initial price of ${} USD",
                amount_to_string(*amount),
                address_to_string(*token),
                amount_to_string(*intial_price),
            )),
            Action::AddLiquidity(amount, token) => Ok(format!(
                "Add {} {} to the liquidity pool",
                amount_to_string(*amount),
                address_to_string(*token),
            )),
            Action::RemoveLiquidity(percentage, token) => Ok(format!(
                "Remove {} of my {} from the liquidity pool",
                percentage_to_string(*percentage),
                address_to_string(*token),
            )),
            Action::Harvest() => Ok(format!("Harvest")),
            Action::Migrate(legacy_address, legacy_signature) => Ok(format!(
                "Migrate {} Signature: {}",
                base64::encode_config(legacy_address, base64::URL_SAFE_NO_PAD),
                base64::encode_config(legacy_signature, base64::URL_SAFE_NO_PAD)
            )),
            Action::CreateRedeemRequest(amount, token) => Ok(format!(
                "Redeem {} {}",
                amount_to_string(*amount),
                address_to_string(*token),
            )),
            Action::Transfer(amount, token, recipient) => Ok(format!(
                "Transfer {} {} to {}",
                amount_to_string(*amount),
                address_to_string(*token),
                address_to_string(*recipient)
            )),
            Action::Trade(input_amount, input_token, minimum_output_amount, output_token) => {
                Ok(format!(
                    "Trade {} {} for at least {} {}",
                    amount_to_string(*input_amount),
                    address_to_string(*input_token),
                    amount_to_string(*minimum_output_amount),
                    address_to_string(*output_token)
                ))
            }
            _ => bail!("Unknown transaction type"),
        }
    }
}

fn percentage_to_string(n: u64) -> String {
    let decimal = n * 100 % BASE_FACTOR;
    let number = (n * 100 - decimal) / BASE_FACTOR;
    format!("{}.{:0>4}%", number, decimal)
}

fn amount_to_string(n: u64) -> String {
    let decimal = n % BASE_FACTOR;
    let number = (n - decimal) / BASE_FACTOR;
    let mut number_parts = number
        .to_string()
        .chars()
        .collect::<Vec<char>>()
        .rchunks(3)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<String>>();
    number_parts.reverse();
    format!("{}.{:0>6}", number_parts.join(","), decimal)
}

pub fn address_to_string(address: Address) -> String {
    match address {
        BTC => return "BTC".to_string(),
        ELC => return "ELC".to_string(),
        ETH => return "ETH".to_string(),
        USD => return "USD".to_string(),
        _ => (),
    };

    let _address_hash = {
        let mut hasher = Keccak256::new();
        hasher.update(address.to_vec());
        hasher.finalize()
    };

    let mut hasher = Keccak256::new();
    hasher.update(hex::encode(address.clone()));
    let address_hash_bytes = hasher.finalize();
    let address_hash = hex::encode(address_hash_bytes);

    let mut address_str = "0x".to_string();
    for (index, address_char) in hex::encode(address).char_indices() {
        let n = u16::from_str_radix(&address_hash[index..index + 1], 16).unwrap();

        if n > 7 {
            address_str.push_str(&address_char.to_uppercase().to_string())
        } else {
            address_str.push(address_char)
        }
    }
    address_str
}

impl VerificationString for Transaction<Action> {
    fn verification_string(&self) -> Result<String> {
        Ok(format!(
            "Network ID: {}\nTransaction Number: {}\nAction: {}",
            self.network_id,
            self.transaction_number,
            self.action.verification_string()?
        ))
    }
}

const PREFIX: &str = "\x19Ethereum Signed Message:\n";

pub fn recover_signed_message(message: &str, signature: &[u8]) -> Result<Address> {
    ecrecover(
        [PREFIX, message.len().to_string().as_str(), message]
            .concat()
            .into_bytes(),
        signature,
    )
}
pub fn ecrecover(hash: Vec<u8>, signature_bytes_slice: &[u8]) -> Result<Address> {
    let mut signature_bytes = signature_bytes_slice.to_vec();
    // See: https://eips.ethereum.org/EIPS/eip-155
    signature_bytes[SIGNATURE_LENGTH - 1] -= 27;
    let signature = recoverable::Signature::try_from(&signature_bytes[..])
        .map_err(|err| anyhow!(err.to_string()))?;
    let public_key = signature
        .recover_verify_key(&hash)
        .map_err(|err| anyhow!(err.to_string()))?;
    eth_address(public_key)[..ADDRESS_LENGTH]
        .try_into()
        .map_err(|e: TryFromSliceError| anyhow!(e.to_string()))
}

pub fn eth_address(verify_key: VerifyingKey) -> Vec<u8> {
    keccak256(verify_key.to_encoded_point(false).to_bytes().to_vec()[1..].to_vec())[12..].to_vec()
}

pub fn keccak256(bytes: Vec<u8>) -> Vec<u8> {
    let mut hasher = Keccak256::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}
