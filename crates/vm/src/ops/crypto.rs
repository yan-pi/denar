//! Signature opcodes: `bip340_verify`.

use secp256k1::schnorr::Signature;
use secp256k1::{Message, Secp256k1, XOnlyPublicKey};

use crate::error::EvalError;
use crate::ops::{boolean, expect_arity};
use crate::value::Value;

/// `(bip340_verify K M S)` — BIP340 Schnorr verification (spec §4).
///
/// An empty signature `S` yields `0` (false) without failing. A well-formed but
/// invalid signature fails the program; a malformed key/message/signature is a
/// type error.
pub(crate) fn bip340_verify(args: &[Value]) -> Result<Value, EvalError> {
    expect_arity("bip340_verify", args, 3)?;

    let key_bytes = args[0]
        .as_atom_bytes()
        .ok_or(EvalError::SchnorrInput("public key must be an atom"))?;
    let msg_bytes = args[1]
        .as_atom_bytes()
        .ok_or(EvalError::SchnorrInput("message must be an atom"))?;
    let sig_bytes = args[2]
        .as_atom_bytes()
        .ok_or(EvalError::SchnorrInput("signature must be an atom"))?;

    // Empty signature -> false (0), per the table's "nil S -> 0".
    if sig_bytes.is_empty() {
        return Ok(Value::Nil);
    }

    let pubkey = XOnlyPublicKey::from_slice(key_bytes)
        .map_err(|_| EvalError::SchnorrInput("public key must be 32 bytes"))?;
    let message = Message::from_digest_slice(msg_bytes)
        .map_err(|_| EvalError::SchnorrInput("message must be 32 bytes"))?;
    let signature = Signature::from_slice(sig_bytes)
        .map_err(|_| EvalError::SchnorrInput("signature must be 64 bytes"))?;

    let secp = Secp256k1::verification_only();
    match secp.verify_schnorr(&signature, &message, &pubkey) {
        Ok(()) => Ok(boolean(true)),
        Err(_) => Err(EvalError::SignatureVerifyFailed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hx(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    // BIP340 reference test vector index 0.
    const PK: &str = "F9308A019258C31049344F85F89D5229B531C845836F99B08601F113BCE036F9";
    const MSG: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    const SIG: &str = "E907831F80848D1069A5371B402410364BDF1C5F8307B0084C55F1CE2DCA821525F66A4A85EA8B71E482A74F382D2CE5EBEEE8FDB2172F477DF4900D310536C0";

    #[test]
    fn verifies_valid_signature() {
        let args = [
            Value::atom(hx(PK)),
            Value::atom(hx(MSG)),
            Value::atom(hx(SIG)),
        ];
        assert_eq!(bip340_verify(&args).unwrap(), Value::atom(vec![1]));
    }

    #[test]
    fn empty_signature_is_false() {
        let args = [Value::atom(hx(PK)), Value::atom(hx(MSG)), Value::Nil];
        assert_eq!(bip340_verify(&args).unwrap(), Value::Nil);
    }

    #[test]
    fn tampered_signature_fails() {
        let mut sig = hx(SIG);
        sig[0] ^= 0x01;
        let args = [Value::atom(hx(PK)), Value::atom(hx(MSG)), Value::atom(sig)];
        assert_eq!(bip340_verify(&args), Err(EvalError::SignatureVerifyFailed));
    }

    #[test]
    fn malformed_pubkey_is_input_error() {
        let args = [
            Value::atom(vec![0x00; 4]),
            Value::atom(hx(MSG)),
            Value::atom(hx(SIG)),
        ];
        assert!(matches!(
            bip340_verify(&args),
            Err(EvalError::SchnorrInput(_))
        ));
    }
}
