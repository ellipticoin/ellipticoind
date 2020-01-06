use tiny_keccak::Hasher;
use tiny_keccak::Keccak;
const SIGNING_PREFIX: &'static str = "\x19Ethereum Signed Message:\n";
pub fn ecrecover_address(message: &[u8], signature: &[u8]) -> Vec<u8> {
    let mut message_hash = [0u8; 32];
    let padded_message = [
        SIGNING_PREFIX.as_bytes(),
        message.len().to_string().as_bytes(),
        message,
    ]
    .concat();
    message_hash.copy_from_slice(&keccak(&padded_message)[..]);
    let message = secp256k1::Message::parse(&message_hash);
    let mut fixed_size_signature: [u8; 64] = [0; 64];
    fixed_size_signature.copy_from_slice(&signature[..64]);
    let signature_parsed = secp256k1::Signature::parse(&fixed_size_signature);
    let recovery_id = secp256k1::RecoveryId::parse(signature[64] - 27 as u8).unwrap();
    let public_key = secp256k1::recover(&message, &signature_parsed, &recovery_id).unwrap();
    public_key_to_address(public_key)
}

fn public_key_to_address(public_key: secp256k1::PublicKey) -> Vec<u8> {
    keccak(&public_key.serialize()[1..])[12..].to_vec()
}

fn keccak(message: &[u8]) -> Vec<u8> {
    let mut hash = [0u8; 32];
    let mut hasher = tiny_keccak::Keccak::v256();
    hasher.update(message);
    hasher.finalize(&mut hash);
    hash.to_vec()
}
