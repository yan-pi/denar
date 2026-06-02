//! Name resolution support: the function table, env-path arithmetic, and
//! lexical scopes.
//!
//! Environment paths follow the Chia/CLVM convention: a number whose leading
//! `1` bit is a terminator, with subsequent bits consumed from the least
//! significant end (`0` = head, `1` = tail). The `i`-th element of a flat env
//! list therefore lives at [`param_path`]`(i)`.

use std::collections::{HashMap, HashSet};

use crate::ast::{Expr, Form, Span};
use crate::error::Error;

/// The env path of the `index`-th element in a flat environment list.
///
/// Position 0 is `0b10` (`head`), position 1 is `0b101` (`head (tail)`), etc.
///
/// # Example
///
/// ```
/// use btclisp_compiler::resolve::param_path;
///
/// assert_eq!(param_path(0), 2);
/// assert_eq!(param_path(1), 5);
/// assert_eq!(param_path(2), 11);
/// ```
#[must_use]
pub fn param_path(index: usize) -> u64 {
    (2u64 << index) | ((1u64 << index) - 1)
}

/// Minimal little-endian byte encoding of `n` (trailing zero bytes stripped).
///
/// # Example
///
/// ```
/// use btclisp_compiler::resolve::le_minimal;
///
/// assert_eq!(le_minimal(0), Vec::<u8>::new());
/// assert_eq!(le_minimal(5), vec![5]);
/// assert_eq!(le_minimal(256), vec![0, 1]);
/// ```
#[must_use]
pub fn le_minimal(n: u64) -> Vec<u8> {
    let mut bytes = n.to_le_bytes().to_vec();
    while bytes.last() == Some(&0) {
        bytes.pop();
    }
    bytes
}

/// A resolved top-level function definition.
#[derive(Clone, Debug)]
pub struct Function {
    /// Formal parameter names, in declaration order.
    pub params: Vec<String>,
    /// The function body expression.
    pub body: Expr,
    /// The span of the whole `defun`.
    pub span: Span,
}

/// All top-level functions, keyed by name.
#[derive(Clone, Debug, Default)]
pub struct FunctionTable {
    map: HashMap<String, Function>,
}

impl FunctionTable {
    /// Collect every `defun` in `forms`, validating names and parameters.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DuplicateFunction`] or [`Error::DuplicateParameter`].
    pub fn collect(forms: &[Form]) -> Result<Self, Error> {
        let mut map = HashMap::new();
        for form in forms {
            let Form::Defun(defun) = form else { continue };
            let mut seen = HashSet::new();
            for param in &defun.params {
                if !seen.insert(param.name.clone()) {
                    return Err(Error::DuplicateParameter {
                        span: param.span,
                        name: param.name.clone(),
                    });
                }
            }
            if map.contains_key(&defun.name.name) {
                return Err(Error::DuplicateFunction {
                    span: defun.name.span,
                    name: defun.name.name.clone(),
                });
            }
            map.insert(
                defun.name.name.clone(),
                Function {
                    params: defun.params.iter().map(|p| p.name.clone()).collect(),
                    body: defun.body.clone(),
                    span: defun.span,
                },
            );
        }
        Ok(Self { map })
    }

    /// Look up a function by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Function> {
        self.map.get(name)
    }

    /// Returns `true` if a function with this name exists.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    /// All function names (unspecified order).
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.map.keys()
    }
}

/// A lexical scope mapping identifiers to environment positions.
#[derive(Clone, Debug)]
pub enum Scope {
    /// A closed scope: only the listed names are visible (`defun`/`let` body).
    Closed(HashMap<String, usize>),
    /// The top-level scope: free identifiers become implicit positional params.
    Top {
        /// Implicit parameter names, in first-appearance order.
        order: Vec<String>,
        /// Reverse index from name to position.
        index: HashMap<String, usize>,
    },
}

impl Scope {
    /// An empty top-level scope.
    #[must_use]
    pub fn top() -> Self {
        Scope::Top {
            order: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// A closed scope binding `names` at consecutive positions.
    #[must_use]
    pub fn closed(names: &[String]) -> Self {
        let index = names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i))
            .collect();
        Scope::Closed(index)
    }

    /// Returns `true` for the top-level scope.
    #[must_use]
    pub fn is_top(&self) -> bool {
        matches!(self, Scope::Top { .. })
    }

    /// Resolve `name` to its env path without mutating the scope.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<u64> {
        match self {
            Scope::Closed(index) | Scope::Top { index, .. } => {
                index.get(name).copied().map(param_path)
            }
        }
    }

    /// Register `name` as a new top-level implicit parameter and return its path.
    ///
    /// Only meaningful for [`Scope::Top`]; a no-op identity path for closed
    /// scopes (callers guard with [`Scope::is_top`]).
    pub fn bind_top(&mut self, name: &str) -> u64 {
        match self {
            Scope::Top { order, index } => {
                let position = order.len();
                order.push(name.to_string());
                index.insert(name.to_string(), position);
                param_path(position)
            }
            Scope::Closed(_) => param_path(0),
        }
    }

    /// Consume the scope, yielding the ordered implicit parameter names.
    #[must_use]
    pub fn into_top_order(self) -> Vec<String> {
        match self {
            Scope::Top { order, .. } => order,
            Scope::Closed(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_program;

    fn table(src: &str) -> FunctionTable {
        FunctionTable::collect(&parse_program(src).unwrap()).unwrap()
    }

    #[test]
    fn param_paths_match_spec_examples() {
        assert_eq!(param_path(0), 2);
        assert_eq!(param_path(1), 5);
        assert_eq!(param_path(2), 11);
        assert_eq!(param_path(3), 23);
    }

    #[test]
    fn le_minimal_strips_trailing_zeros() {
        assert_eq!(le_minimal(0), Vec::<u8>::new());
        assert_eq!(le_minimal(1), vec![1]);
        assert_eq!(le_minimal(11), vec![11]);
        assert_eq!(le_minimal(256), vec![0, 1]);
    }

    #[test]
    fn collects_functions() {
        let t = table("(defun f (a b) (+ a b))\n(defun g () 1)\n(f 1 2)");
        assert!(t.contains("f"));
        assert!(t.contains("g"));
        assert_eq!(t.get("f").unwrap().params, vec!["a", "b"]);
    }

    #[test]
    fn rejects_duplicate_function() {
        let err = FunctionTable::collect(&parse_program("(defun f () 1)\n(defun f () 2)").unwrap())
            .unwrap_err();
        assert!(matches!(err, Error::DuplicateFunction { .. }));
    }

    #[test]
    fn rejects_duplicate_parameter() {
        let err = FunctionTable::collect(&parse_program("(defun f (a a) a)").unwrap()).unwrap_err();
        assert!(matches!(err, Error::DuplicateParameter { .. }));
    }

    #[test]
    fn closed_scope_resolves_positions() {
        let s = Scope::closed(&["a".to_string(), "b".to_string()]);
        assert_eq!(s.get("a"), Some(2));
        assert_eq!(s.get("b"), Some(5));
        assert_eq!(s.get("c"), None);
    }

    #[test]
    fn top_scope_binds_in_order() {
        let mut s = Scope::top();
        assert_eq!(s.bind_top("x"), 2);
        assert_eq!(s.bind_top("y"), 5);
        assert_eq!(s.get("x"), Some(2));
        assert_eq!(s.into_top_order(), vec!["x", "y"]);
    }
}
