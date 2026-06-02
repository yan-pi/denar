//! Lowering from the surface AST to `btclisp-core`'s [`CoreExpr`].
//!
//! The pipeline mirrors spec §6: collect functions, then lower the single
//! top-level program expression, expanding `let`/`if`/`cond` and inlining
//! non-recursive function calls. Environments are flat lists; see
//! [`crate::resolve`] for path arithmetic.

use std::collections::{HashMap, HashSet};

use btclisp_core::{CoreExpr, Opcode};

use crate::ast::{Atom, Binding, Clause, Expr, Form, Span, Symbol};
use crate::error::Error;
use crate::resolve::{le_minimal, FunctionTable, Scope};

/// A lowered program: its core expression and the implicit env signature.
#[derive(Clone, Debug)]
pub struct Program {
    /// The lowered core expression.
    pub core: CoreExpr,
    /// Top-level free identifiers, in the order they must appear in the env.
    pub env_params: Vec<String>,
}

/// Lower a parsed program into core form.
///
/// # Errors
///
/// Returns any resolution or lowering [`Error`]; every `defun` body is
/// validated even if uncalled.
pub fn lower_program(forms: &[Form]) -> Result<Program, Error> {
    let table = FunctionTable::collect(forms)?;
    let mut lowerer = Lowerer::new(&table);
    lowerer.validate_all_functions()?;

    let mut mains = forms.iter().filter_map(|form| match form {
        Form::Expr(expr) => Some(expr),
        Form::Defun(_) => None,
    });
    let main = mains.next().ok_or(Error::NoProgramExpression)?;
    if let Some(extra) = mains.next() {
        return Err(Error::MultipleProgramExpressions { span: extra.span() });
    }

    let mut scope = Scope::top();
    let core = lowerer.lower_expr(main, &mut scope)?;
    Ok(Program {
        core,
        env_params: scope.into_top_order(),
    })
}

/// Stateful lowering driver: memoises function bodies and guards recursion.
struct Lowerer<'a> {
    table: &'a FunctionTable,
    bodies: HashMap<String, CoreExpr>,
    in_progress: Vec<String>,
}

impl<'a> Lowerer<'a> {
    fn new(table: &'a FunctionTable) -> Self {
        Self {
            table,
            bodies: HashMap::new(),
            in_progress: Vec::new(),
        }
    }

    /// Lower (and thereby validate) every function body once.
    fn validate_all_functions(&mut self) -> Result<(), Error> {
        let names: Vec<String> = self.table.names().cloned().collect();
        for name in &names {
            let span = match self.table.get(name) {
                Some(func) => func.span,
                None => continue,
            };
            self.function_body(name, span)?;
        }
        Ok(())
    }

    /// Return the lowered body of function `name`, inlining-ready and memoised.
    fn function_body(&mut self, name: &str, call_span: Span) -> Result<CoreExpr, Error> {
        let table = self.table;
        let func = match table.get(name) {
            Some(func) => func,
            None => {
                return Err(Error::UnknownFunction {
                    span: call_span,
                    name: name.to_string(),
                })
            }
        };
        if let Some(body) = self.bodies.get(name) {
            return Ok(body.clone());
        }
        if self.in_progress.iter().any(|n| n == name) {
            return Err(Error::RecursiveCall {
                span: call_span,
                name: name.to_string(),
            });
        }

        self.in_progress.push(name.to_string());
        let mut scope = Scope::closed(&func.params);
        let body = self.lower_expr(&func.body, &mut scope)?;
        self.in_progress.pop();
        self.bodies.insert(name.to_string(), body.clone());
        Ok(body)
    }

    fn lower_expr(&mut self, expr: &Expr, scope: &mut Scope) -> Result<CoreExpr, Error> {
        match expr {
            Expr::Atom { atom, span } => self.lower_atom(atom, *span, scope),
            Expr::Quote { inner, .. } => Ok(core_quote(lower_datum(inner)?)),
            Expr::If {
                cond, then, els, ..
            } => {
                let c = self.lower_expr(cond, scope)?;
                let t = self.lower_expr(then, scope)?;
                let e = self.lower_expr(els, scope)?;
                Ok(core_if(c, t, e))
            }
            Expr::Let {
                bindings,
                body,
                span,
            } => self.lower_let(bindings, body, *span, scope),
            Expr::Cond { clauses, .. } => self.lower_cond(clauses, scope),
            Expr::App { head, args, span } => self.lower_app(head, args, *span, scope),
        }
    }

    fn lower_atom(&self, atom: &Atom, span: Span, scope: &mut Scope) -> Result<CoreExpr, Error> {
        match atom {
            Atom::Int(n) => Ok(core_quote(int_value(*n))),
            Atom::Hex(bytes) => Ok(core_quote(CoreExpr::atom(bytes.clone()))),
            Atom::Str(s) => Ok(core_quote(CoreExpr::atom(s.clone().into_bytes()))),
            Atom::Nil => Ok(core_quote(CoreExpr::Nil)),
            Atom::Ident(name) => self.lower_ident(name, span, scope),
        }
    }

    fn lower_ident(&self, name: &str, span: Span, scope: &mut Scope) -> Result<CoreExpr, Error> {
        if let Some(path) = scope.get(name) {
            return Ok(env_ref(path));
        }
        if self.table.contains(name) {
            return Err(Error::FunctionAsValue {
                span,
                name: name.to_string(),
            });
        }
        if scope.is_top() {
            return Ok(env_ref(scope.bind_top(name)));
        }
        Err(Error::UndefinedIdentifier {
            span,
            name: name.to_string(),
        })
    }

    fn lower_let(
        &mut self,
        bindings: &[Binding],
        body: &[Expr],
        span: Span,
        scope: &mut Scope,
    ) -> Result<CoreExpr, Error> {
        if body.len() != 1 {
            return Err(Error::UnsupportedLetBody { span });
        }
        let mut seen = HashSet::new();
        for binding in bindings {
            if !seen.insert(binding.name.name.clone()) {
                return Err(Error::DuplicateBinding {
                    span: binding.name.span,
                    name: binding.name.name.clone(),
                });
            }
        }

        // Binding values are evaluated in the *outer* scope.
        let mut values = Vec::with_capacity(bindings.len());
        for binding in bindings {
            values.push(self.lower_expr(&binding.value, scope)?);
        }

        let names: Vec<String> = bindings.iter().map(|b| b.name.name.clone()).collect();
        let mut inner = Scope::closed(&names);
        let body_core = self.lower_expr(&body[0], &mut inner)?;
        Ok(make_apply(body_core, values))
    }

    fn lower_cond(&mut self, clauses: &[Clause], scope: &mut Scope) -> Result<CoreExpr, Error> {
        let mut acc = core_app(Opcode::X, Vec::new());
        for clause in clauses.iter().rev() {
            let test = self.lower_expr(&clause.test, scope)?;
            let result = self.lower_expr(&clause.result, scope)?;
            acc = core_if(test, result, acc);
        }
        Ok(acc)
    }

    fn lower_app(
        &mut self,
        head: &Symbol,
        args: &[Expr],
        span: Span,
        scope: &mut Scope,
    ) -> Result<CoreExpr, Error> {
        let name = head.name.as_str();

        if name == "/" {
            if args.len() != 2 {
                return Err(Error::ArityMismatch {
                    span,
                    name: "/".to_string(),
                    expected: 2,
                    found: args.len(),
                });
            }
            let dividend = self.lower_expr(&args[0], scope)?;
            let divisor = self.lower_expr(&args[1], scope)?;
            let divmod = core_app(Opcode::DivMod, vec![dividend, divisor]);
            return Ok(core_app(Opcode::H, vec![divmod]));
        }

        if let Some(op) = core_op(name) {
            let lowered = self.lower_args(args, scope)?;
            return Ok(core_app(op, lowered));
        }

        if let Some(arity) = self.table.get(name).map(|f| f.params.len()) {
            if args.len() != arity {
                return Err(Error::ArityMismatch {
                    span,
                    name: name.to_string(),
                    expected: arity,
                    found: args.len(),
                });
            }
            let lowered = self.lower_args(args, scope)?;
            let body = self.function_body(name, head.span)?;
            return Ok(make_apply(body, lowered));
        }

        Err(Error::UnknownFunction {
            span: head.span,
            name: name.to_string(),
        })
    }

    fn lower_args(&mut self, args: &[Expr], scope: &mut Scope) -> Result<Vec<CoreExpr>, Error> {
        let mut lowered = Vec::with_capacity(args.len());
        for arg in args {
            lowered.push(self.lower_expr(arg, scope)?);
        }
        Ok(lowered)
    }
}

/// Map a surface operator name to its core [`Opcode`], if it is one.
///
/// `/` is handled separately (it expands via `/%`), and `if` is surface sugar
/// intercepted by the parser, so neither appears here.
fn core_op(name: &str) -> Option<Opcode> {
    let op = match name {
        "q" => Opcode::Q,
        "a" => Opcode::A,
        "x" => Opcode::X,
        "c" => Opcode::C,
        "h" => Opcode::H,
        "t" => Opcode::T,
        "l" => Opcode::L,
        "b" => Opcode::BinTree,
        "not" => Opcode::Not,
        "all" => Opcode::All,
        "any" => Opcode::Any,
        "=" => Opcode::Eq,
        "<" => Opcode::Lt,
        "+" => Opcode::Add,
        "-" => Opcode::Sub,
        "*" => Opcode::Mul,
        "%" => Opcode::Mod,
        "cat" => Opcode::Cat,
        "substr" => Opcode::Substr,
        "strlen" => Opcode::Strlen,
        "sha256" => Opcode::Sha256,
        "hash160" => Opcode::Hash160,
        "hash256" => Opcode::Hash256,
        "bip340_verify" => Opcode::Bip340Verify,
        "tx" => Opcode::Tx,
        "bip342_txmsg" => Opcode::Bip342Txmsg,
        "rd" => Opcode::Rd,
        "wr" => Opcode::Wr,
        _ => return None,
    };
    Some(op)
}

/// Convert a quoted surface expression into its core data structure.
fn lower_datum(expr: &Expr) -> Result<CoreExpr, Error> {
    match expr {
        Expr::Atom { atom, .. } => Ok(datum_atom(atom)),
        Expr::App { head, args, .. } => {
            let mut items = Vec::with_capacity(args.len() + 1);
            items.push(datum_ident(&head.name));
            for arg in args {
                items.push(lower_datum(arg)?);
            }
            Ok(proper_list(items))
        }
        Expr::Quote { inner, .. } => {
            let datum = lower_datum(inner)?;
            Ok(proper_list(vec![opcode_atom(Opcode::Q), datum]))
        }
        Expr::If { span, .. } | Expr::Let { span, .. } | Expr::Cond { span, .. } => {
            Err(Error::UnquotableForm { span: *span })
        }
    }
}

fn datum_atom(atom: &Atom) -> CoreExpr {
    match atom {
        Atom::Int(n) => int_value(*n),
        Atom::Hex(bytes) => CoreExpr::atom(bytes.clone()),
        Atom::Str(s) => CoreExpr::atom(s.clone().into_bytes()),
        Atom::Nil => CoreExpr::Nil,
        Atom::Ident(name) => datum_ident(name),
    }
}

fn datum_ident(name: &str) -> CoreExpr {
    match core_op(name) {
        Some(op) => opcode_atom(op),
        None => CoreExpr::atom(name.as_bytes().to_vec()),
    }
}

// --- core builders ---------------------------------------------------------

fn opcode_atom(op: Opcode) -> CoreExpr {
    CoreExpr::atom(vec![op.as_byte()])
}

fn env_ref(path: u64) -> CoreExpr {
    CoreExpr::atom(le_minimal(path))
}

fn int_value(n: i64) -> CoreExpr {
    let bytes = le_minimal(n as u64);
    if bytes.is_empty() {
        CoreExpr::Nil
    } else {
        CoreExpr::atom(bytes)
    }
}

/// Quote a core value: `(q . value)`.
fn core_quote(value: CoreExpr) -> CoreExpr {
    CoreExpr::cons(opcode_atom(Opcode::Q), value)
}

/// Build the application `(op arg0 arg1 ...)`.
fn core_app(op: Opcode, args: Vec<CoreExpr>) -> CoreExpr {
    CoreExpr::cons(opcode_atom(op), proper_list(args))
}

/// Build a nil-terminated proper list from `items`.
fn proper_list(items: Vec<CoreExpr>) -> CoreExpr {
    let mut list = CoreExpr::Nil;
    for item in items.into_iter().rev() {
        list = CoreExpr::cons(item, list);
    }
    list
}

/// Build the environment-constructing expression `(c a0 (c a1 ... (q)))`.
fn build_env(args: Vec<CoreExpr>) -> CoreExpr {
    let mut acc = core_quote(CoreExpr::Nil);
    for arg in args.into_iter().rev() {
        acc = core_app(Opcode::C, vec![arg, acc]);
    }
    acc
}

/// `(a (q . body) <env from args>)` — evaluate `body` with a fresh env.
fn make_apply(body: CoreExpr, args: Vec<CoreExpr>) -> CoreExpr {
    core_app(Opcode::A, vec![core_quote(body), build_env(args)])
}

/// `(a (i C (q . T) (q . E)) 1)` — run the selected branch in the current env.
fn core_if(cond: CoreExpr, then: CoreExpr, els: CoreExpr) -> CoreExpr {
    let select = core_app(Opcode::If, vec![cond, core_quote(then), core_quote(els)]);
    core_app(Opcode::A, vec![select, env_ref(1)])
}

#[cfg(test)]
mod tests {
    use crate::compile;

    fn core(src: &str) -> String {
        compile(src).unwrap().core.to_sexpr()
    }

    #[test]
    fn lowers_addition() {
        assert_eq!(core("(+ 2 3)"), "(0x17 (0x00 . 0x02) (0x00 . 0x03))");
    }

    #[test]
    fn lowers_nested_arithmetic() {
        assert_eq!(
            core("(+ (* 2 3) 4)"),
            "(0x17 (0x19 (0x00 . 0x02) (0x00 . 0x03)) (0x00 . 0x04))"
        );
    }

    #[test]
    fn top_level_free_vars_become_env_refs() {
        let program = crate::compile("(bip340_verify pk msg sig)").unwrap();
        assert_eq!(program.core.to_sexpr(), "(0x28 0x02 0x05 0x0b)");
        assert_eq!(program.env_params, vec!["pk", "msg", "sig"]);
    }

    #[test]
    fn lowers_let_to_apply() {
        assert_eq!(
            core("(let ((x 1) (y 2)) (+ x y))"),
            "(0x01 (0x00 0x17 0x02 0x05) \
             (0x05 (0x00 . 0x01) (0x05 (0x00 . 0x02) (0x00))))"
        );
    }

    #[test]
    fn lowers_if_to_quoted_branches() {
        assert_eq!(
            core("(if 1 2 3)"),
            "(0x01 (0x03 (0x00 . 0x01) (0x00 0x00 . 0x02) (0x00 0x00 . 0x03)) 0x01)"
        );
    }

    #[test]
    fn lowers_division_via_divmod() {
        assert_eq!(core("(/ 7 2)"), "(0x06 (0x1b (0x00 . 0x07) (0x00 . 0x02)))");
    }

    #[test]
    fn inlines_function_call() {
        assert_eq!(
            core("(defun sq (n) (* n n))\n(sq 4)"),
            "(0x01 (0x00 0x19 0x02 0x02) (0x05 (0x00 . 0x04) (0x00)))"
        );
    }

    #[test]
    fn lowers_cond_to_nested_ifs() {
        // (cond (1 2)) -> (a (i (q.1) (q.(q.2)) (q.(x))) 1)
        assert_eq!(
            core("(cond (1 2))"),
            "(0x01 (0x03 (0x00 . 0x01) (0x00 0x00 . 0x02) (0x00 0x02)) 0x01)"
        );
    }

    #[test]
    fn lowers_quote_sugar_to_data() {
        assert_eq!(core("'(+ 1 2)"), "(0x00 0x17 0x01 0x02)");
    }

    #[test]
    fn rejects_undefined_ident_in_function_body() {
        let err = crate::compile("(defun f (a) (+ a b))\n(f 1)").unwrap_err();
        assert!(matches!(err, crate::Error::UndefinedIdentifier { .. }));
    }

    #[test]
    fn rejects_arity_mismatch() {
        let err = crate::compile("(defun f (a) a)\n(f 1 2)").unwrap_err();
        assert!(matches!(err, crate::Error::ArityMismatch { .. }));
    }

    #[test]
    fn rejects_recursive_call() {
        let err = crate::compile("(defun f (n) (f n))\n(f 1)").unwrap_err();
        assert!(matches!(err, crate::Error::RecursiveCall { .. }));
    }

    #[test]
    fn rejects_function_as_value() {
        let err = crate::compile("(defun f () 1)\n(+ f 1)").unwrap_err();
        assert!(matches!(err, crate::Error::FunctionAsValue { .. }));
    }

    #[test]
    fn rejects_unknown_function() {
        let err = crate::compile("(frobnicate 1)").unwrap_err();
        assert!(matches!(err, crate::Error::UnknownFunction { .. }));
    }

    #[test]
    fn rejects_program_without_main() {
        let err = crate::compile("(defun f () 1)").unwrap_err();
        assert!(matches!(err, crate::Error::NoProgramExpression));
    }

    #[test]
    fn rejects_multiple_main_expressions() {
        let err = crate::compile("1\n2").unwrap_err();
        assert!(matches!(
            err,
            crate::Error::MultipleProgramExpressions { .. }
        ));
    }
}
