# Demo btclisp para apresentação

Use estes arquivos para demonstrar o pipeline do zero, sem depender do corpus em
`examples/`.

## Check + core

```sh
cargo run -p btclisp-cli -- check --core demo/presentation/01_add.btl
```

## Compile

```sh
cargo run -p btclisp-cli -- compile demo/presentation/01_add.btl -o /tmp/btclisp-add.bin
```

## Disassemble

```sh
cargo run -p btclisp-cli -- disasm /tmp/btclisp-add.bin
```

## Run

```sh
cargo run -p btclisp-cli -- run /tmp/btclisp-add.bin
```

Resultado esperado: `0x2a` (`42`).

## Recursão

```sh
cargo run -p btclisp-cli -- check --core demo/presentation/02_factorial.btl
cargo run -p btclisp-cli -- compile demo/presentation/02_factorial.btl -o /tmp/btclisp-fact.bin
cargo run -p btclisp-cli -- disasm /tmp/btclisp-fact.bin
cargo run -p btclisp-cli -- run /tmp/btclisp-fact.bin
```

Resultado esperado: `0x78` (`120`).

## `let` + `if`

```sh
cargo run -p btclisp-cli -- check --core demo/presentation/03_let_if.btl
cargo run -p btclisp-cli -- compile demo/presentation/03_let_if.btl -o /tmp/btclisp-let-if.bin
cargo run -p btclisp-cli -- run /tmp/btclisp-let-if.bin
```

Resultado esperado: `0x01` (verdadeiro).
