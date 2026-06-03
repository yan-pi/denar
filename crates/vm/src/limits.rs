//! Resource limits and counters for evaluation.

/// Hard budgets enforced during evaluation.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Maximum number of evaluation steps.
    pub max_ops: u64,
    /// Maximum bytes the program may allocate (advisory in v0,1).
    pub max_alloc_bytes: u64,
    /// Maximum call depth.
    pub max_stack: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_ops: 1_000_000,
            max_alloc_bytes: 5_000_000,
            max_stack: 1_000,
        }
    }
}

/// Running resource usage during an evaluation.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Counters {
    /// Evaluation steps taken so far.
    pub ops: u64,
    /// Bytes allocated so far (advisory in v0,1).
    pub alloc_bytes: u64,
    /// Deepest call depth reached.
    pub max_depth: u32,
}
