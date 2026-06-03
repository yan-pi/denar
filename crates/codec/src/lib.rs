//! Binary codec between `btclisp-core`'s [`btclisp_core::CoreExpr`] and bytes.
//!
//! Implements ajtowns' encoding table (spec §7) as a *canonical* mapping, so
//! that [`decode`]`(`[`encode`]`(e)) == e` for every expression. See
//! [`encode`] for the documented choices on the under-specified codes.
//!
//! # Example
//!
//! ```
//! use btclisp_core::CoreExpr;
//! use btclisp_codec::{decode, encode};
//!
//! let expr = CoreExpr::cons(CoreExpr::atom(vec![0x17]), CoreExpr::Nil);
//! assert_eq!(decode(&encode(&expr)).unwrap(), expr);
//! ```

pub mod decode;
pub mod encode;
pub mod error;

pub use decode::decode;
pub use encode::encode;
pub use error::Error;
