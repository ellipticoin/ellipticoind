use anyhow::{anyhow, bail, Result};
use core::array::TryFromSliceError;
use ellipticoin_contracts::{
    constants::{BASE_FACTOR, BTC, ETH, MS, LEVERAGED_BASE_TOKEN},
    governance::Vote,
    order_book::OrderType,
    Action, Transaction,
};
use ellipticoin_types::{
    db::{Backend, Db},
    traits::Run,
    Address, ADDRESS_LENGTH,
};
use k256::{
    ecdsa::{recoverable, VerifyingKey},
    elliptic_curve::sec1::ToEncodedPoint,
};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::convert::{TryFrom, TryInto};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedTransaction(pub Transaction, Vec<u8>);

const SIGNATURE_LENGTH: usize = 65;

impl Run for SignedTransaction {
    fn sender(&self) -> Result<[u8; 20]> {
        recover_signed_message(&self.0.verification_string()?, &self.1)
    }

    fn run<B: Backend>(&self, db: &mut Db<B>) -> Result<()> {
        self.0.run(db, self.sender()?)
    }
}

pub trait VerificationString {
    fn verification_string(&self) -> Result<String>;
}

impl VerificationString for Action {
    fn verification_string(&self) -> Result<String> {
        match &self {
            Action::AddLiquidity(amount, token) => Ok(format!(
                "Add {} {} to the liquidity pool",
                amount_to_string(*amount),
                address_to_string(*token),
            )),
            Action::CreateOrder(order_type, amount, token, price) => {
                return Ok(format!(
                    "Create a limit order to {} {} {} for $ {} a piece",
                    order_type_to_string(order_type.clone()),
                    amount_to_string(*amount),
                    address_to_string(*token),
                    amount_to_string(*price),
                ));
            }
            Action::CreatePool(amount, token, intial_price) => Ok(format!(
                "Create a pool of {} {} at an initial price of ${} USD",
                amount_to_string(*amount),
                address_to_string(*token),
                amount_to_string(*intial_price),
            )),
            Action::CreateProposal(title, subtitle, content, actions) => Ok(format!(
                "Create Proposal\nTitle: {}\nSubtitle: {}\nContent: {}\nActions: {}",
                title,
                subtitle,
                content,
                actions_to_string(actions)?
            )),
            Action::CreateRedeemRequest(amount, token) => Ok(format!(
                "Redeem {} {}",
                amount_to_string(*amount),
                address_to_string(*token),
            )),
            Action::FillOrder(order_id, order_type, amount, token, price) => {
                println!(
                    "{}\nOrder Id: #{}\nToken: {}\nAmount: {}\nPrice: $ {} / {}\nTotal: $ {}",
                    inverted_order_type_to_string(order_type.clone()),
                    order_id,
                    address_to_string(*token),
                    amount_to_string(*amount),
                    amount_to_string(*price),
                    address_to_string(*token),
                    amount_to_string(*amount * *price / BASE_FACTOR),
                );
                return Ok(format!(
                    "{}\nOrder Id: #{}\nToken: {}\nAmount: {}\nPrice: $ {} / {}\nTotal: $ {}",
                    inverted_order_type_to_string(order_type.clone()),
                    order_id,
                    address_to_string(*token),
                    amount_to_string(*amount),
                    amount_to_string(*price),
                    address_to_string(*token),
                    amount_to_string(*amount * *price / BASE_FACTOR),
                ));
            }
            Action::Harvest() => Ok(format!("Harvest")),
            Action::Migrate(legacy_address, legacy_signature) => Ok(format!(
                "Migrate {} Signature: {}",
                base64::encode_config(legacy_address, base64::URL_SAFE_NO_PAD),
                base64::encode_config(legacy_signature, base64::URL_SAFE_NO_PAD)
            )),
            Action::Pay(recipient, amount, token) => Ok(format!(
                "Pay {} {} {}",
                address_to_string(*recipient),
                amount_to_string(*amount),
                address_to_string(*token)
            )),
            Action::RemoveLiquidity(percentage, token) => Ok(format!(
                "Remove {} of my {} from the liquidity pool",
                percentage_to_string(*percentage),
                address_to_string(*token),
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
            Action::Vote(proposal_id, vote) => Ok(format!(
                "Vote {} on MS {}",
                vote_to_string(vote.clone()),
                proposal_id
            )),
            _ => bail!("Unknown transaction type"),
        }
    }
}

fn vote_to_string(vote: Vote) -> String {
    if matches!(vote, Vote::For) {
        "Yes".to_string()
    } else {
        "No".to_string()
    }
}

fn order_type_to_string(order_type: OrderType) -> String {
    format!("{:?}", order_type).to_ascii_lowercase()
}

fn inverted_order_type_to_string(order_type: OrderType) -> String {
    match order_type {
        OrderType::Buy => "Sell".to_string(),
        OrderType::Sell => "Buy".to_string(),
    }
}

fn actions_to_string(actions: &Vec<Action>) -> Result<String> {
    Ok(actions
        .iter()
        .map(|action| action.verification_string())
        .collect::<Result<Vec<String>, _>>()?
        .join("\n"))
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
        MS => return "MS".to_string(),
        ETH => return "ETH".to_string(),
        LEVERAGED_BASE_TOKEN => return "USD".to_string(),
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

impl VerificationString for Transaction {
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

pub fn eth_address(verify_key: VerifyingKey) -> [u8; 20] {
    keccak256(verify_key.to_encoded_point(false).to_bytes().to_vec()[1..].to_vec())[12..]
        .try_into()
        .unwrap()
}

pub fn keccak256(bytes: Vec<u8>) -> Vec<u8> {
    let mut hasher = Keccak256::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}
