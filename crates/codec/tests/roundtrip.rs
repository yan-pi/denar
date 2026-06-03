//! Property-based round-trip tests for the codec (spec §7).
//!
//! Generated expressions are *canonical*: atoms are non-empty (an empty atom
//! and `nil` share the `0x00` encoding, with `nil` being canonical), so the
//! encoder is injective over the generated domain.

use btclisp_codec::{decode, encode};
use btclisp_core::CoreExpr;
use proptest::prelude::*;

/// A strategy producing arbitrary canonical core expressions.
fn core_expr() -> impl Strategy<Value = CoreExpr> {
    // Atom lengths span every codec range: literal (1), short (2..64),
    // 0x34 (65..97), and varint (>=98).
    let atom = prop_oneof![
        prop::collection::vec(any::<u8>(), 1..=4),
        prop::collection::vec(any::<u8>(), 64..=66),
        prop::collection::vec(any::<u8>(), 96..=100),
    ]
    .prop_map(CoreExpr::atom);

    let leaf = prop_oneof![Just(CoreExpr::Nil), atom];

    leaf.prop_recursive(6, 64, 8, |inner| {
        prop_oneof![
            // proper and improper cons cells of varying width
            (inner.clone(), inner.clone()).prop_map(|(h, t)| CoreExpr::cons(h, t)),
            prop::collection::vec(inner, 1..=8).prop_map(|elems| {
                let mut list = CoreExpr::Nil;
                for el in elems.into_iter().rev() {
                    list = CoreExpr::cons(el, list);
                }
                list
            }),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    /// decode(encode(e)) == e for arbitrary canonical expressions.
    #[test]
    fn decode_encode_is_identity(expr in core_expr()) {
        let bytes = encode(&expr);
        let decoded = decode(&bytes).expect("decode of encoded bytes");
        prop_assert_eq!(decoded, expr);
    }

    /// encode(decode(b)) == b for any b in the image of encode.
    #[test]
    fn encode_decode_is_identity(expr in core_expr()) {
        let bytes = encode(&expr);
        let decoded = decode(&bytes).expect("decode of encoded bytes");
        prop_assert_eq!(encode(&decoded), bytes);
    }
}
