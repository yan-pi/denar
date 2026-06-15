# Conteúdo para apresentar o projeto btclisp

Este arquivo é uma “cola técnica” para estudar antes da apresentação. Ele junta
o que você precisa saber para explicar o projeto em uma disciplina de
Compiladores, sem precisar entrar fundo em Rust ou Bitcoin.

## 1. Mensagem principal da apresentação

A mensagem central é:

> btclisp é um exemplo pequeno, mas completo, de pipeline de compilador: ele vai
> do código fonte até uma representação intermediária, serializa essa
> representação em bytecode e executa o resultado em uma VM.

O projeto não é só um parser. Ele tem:

1. Linguagem fonte.
2. Lexer.
3. Parser.
4. AST.
5. Resolução de nomes.
6. Lowering/desugaring.
7. IR (`CoreExpr`).
8. Codec/bytecode.
9. VM/evaluator.
10. Testes unitários, property tests e testes end-to-end.

## 2. O que é btclisp em uma frase

btclisp é uma linguagem Lisp pequena, implementada em Rust, que compila
programas escritos em arquivos `.btl` para uma IR minimalista chamada
`CoreExpr`, serializa essa IR em bytes e executa o programa em uma VM eager.

Para a apresentação, diga:

> O domínio original do projeto é Bitcoin, mas hoje eu vou tratar essa parte
> como contexto. O foco é o pipeline de compilação.

## 3. O que tratar como black box

Você não precisa explicar profundamente:

- Rust.
- Taproot.
- BIP340/BIP342.
- Criptografia.
- Regtest/Nigiri.

Como explicar se perguntarem:

> Bitcoin entra como domínio real de aplicação. Alguns opcodes da VM sabem lidar
> com hash, assinatura e transação, mas a estrutura de compilador independe
> disso. Eu poderia substituir esses opcodes por operações de outro domínio e o
> pipeline continuaria parecido.

## 4. Pipeline completo

Fluxo principal:

```txt
.btl source
  -> lexer
  -> tokens
  -> parser
  -> AST de superfície
  -> resolver/lowering
  -> CoreExpr
  -> codec
  -> bytes
  -> VM
  -> Value
```

Em forma curta:

```txt
texto -> tokens -> AST -> IR -> bytecode -> execução
```

Mapeamento para crates:

| Etapa | Crate/arquivo principal |
|---|---|
| IR/opcodes | `crates/core` |
| Lexer/parser/AST/lowering | `crates/compiler` |
| Encode/decode/disasm | `crates/codec` |
| VM/eval/opcodes | `crates/vm` |
| Interface de linha de comando | `crates/cli` |

## 5. Linguagem fonte

A linguagem usa S-expressions, estilo Lisp.

Exemplo mínimo:

```lisp
(+ 40 2)
```

Exemplo com variável local:

```lisp
(let ((x 40)
      (y 2))
  (+ x y))
```

Exemplo com função e recursão:

```lisp
(defun fact (n)
  (if (< n 2)
      1
      (* n (fact (- n 1)))))

(fact 5)
```

Construções importantes:

- Átomos: inteiros, hex, string, `nil`, identificadores.
- Aplicações: `(<nome> arg1 arg2 ...)`.
- `defun`: definição de função.
- `let`: bindings locais.
- `if`: condicional.
- `cond`: múltiplos condicionais.
- Quote: `'(...)`.

## 6. Lexer

O lexer transforma caracteres em tokens.

Exemplo:

```lisp
(+ 40 2)
```

vira conceitualmente:

```txt
LParen
Sym("+")
Int(40)
Int(2)
RParen
```

O que destacar:

- O lexer ignora espaços e comentários.
- Cada token carrega um `span`, ou seja, posição inicial e final no arquivo.
- `span` é importante para diagnósticos de erro.

Exemplo de tokens reais no código:

- `Tok::LParen`
- `Tok::RParen`
- `Tok::Quote`
- `Tok::Nil`
- `Tok::Int(i64)`
- `Tok::Hex(Vec<u8>)`
- `Tok::Str(String)`
- `Tok::Sym(String)`

Mensagem didática:

> O lexer não entende a árvore do programa; ele só classifica pedaços do texto.

## 7. Parser

O parser transforma tokens em AST.

O parser do projeto é recursivo-descendente. Isso combina bem com Lisp, porque a
gramática é simples: listas começam com `(`, terminam com `)`, e a estrutura já é
bem explícita.

Exemplo:

```lisp
(+ 40 2)
```

vira algo como:

```txt
Expr::App {
  head: "+",
  args: [Int(40), Int(2)]
}
```

O parser entende formas especiais:

- `(let ...)`
- `(if ...)`
- `(cond ...)`
- `(defun ...)`

Mensagem didática:

> Depois do parser, o compilador não precisa mais lidar com texto cru. Ele lida
> com uma árvore estruturada.

## 8. AST de superfície

A AST de superfície representa o que o usuário escreveu.

Nós chamamos de “superfície” porque ela ainda tem açúcar sintático:

- `let`.
- `if`.
- `cond`.
- `defun`.
- nomes de variáveis.
- nomes de funções.

Tipos conceituais importantes:

- `Expr::Atom`
- `Expr::App`
- `Expr::Let`
- `Expr::If`
- `Expr::Cond`
- `Expr::Quote`
- `Form::Defun`

Mensagem didática:

> A AST é confortável para representar a linguagem que o usuário escreve, mas
> ainda é grande demais para a VM. Por isso existe a fase de lowering.

## 9. Resolução de nomes

Antes de executar, nomes precisam desaparecer.

Na linguagem fonte, o usuário escreve:

```lisp
(+ a b)
```

Mas a VM não trabalha com os nomes `a` e `b`. O compilador transforma nomes em
referências posicionais no ambiente.

Exemplo conceitual:

```txt
a -> caminho 2
b -> caminho 5
c -> caminho 11
```

Ambiente flat:

```txt
(a . (b . (c . nil)))
```

Isso é parecido com o que compiladores fazem quando transformam nomes em offsets
ou endereços.

Mensagem didática:

> Nome é uma preocupação do frontend. Runtime prefere posições, caminhos ou
> endereços.

## 10. Lowering / desugaring

Lowering é a fase mais importante para explicar.

Ela transforma a linguagem de superfície em uma linguagem núcleo menor.

Exemplos:

### Inteiros viram dados quotados

Fonte:

```lisp
40
```

Core conceitual:

```txt
(q . 0x28)
```

### Aplicação simples

Fonte:

```lisp
(+ 40 2)
```

Core conceitual:

```txt
(+ (q . 0x28) (q . 0x02))
```

### `let`

Fonte:

```lisp
(let ((x 40)
      (y 2))
  (+ x y))
```

Ideia do lowering:

1. Avalia os valores dos bindings.
2. Constrói um ambiente novo.
3. Aplica o corpo nesse ambiente.

Mensagem:

> `let` não precisa existir na VM. Ele é açúcar que o compilador reescreve.

### `if`

O `if` precisa tomar cuidado para não avaliar os dois branches.

Por isso o lowering usa branches quotados.

Mensagem:

> O if é interessante porque mostra que lowering também preserva semântica, não
> só muda sintaxe.

### Funções

Em v0.1, chamadas de função são inlined/baixadas para aplicação com ambiente.

Recursão direta é suportada por uma técnica chamada auto-quine, em que a função
carrega uma referência ao próprio programa.

Não precisa explicar auto-quine em profundidade. Basta dizer:

> Para permitir recursão direta, o lowering coloca uma cópia/referência do corpo
> da função dentro do ambiente, então a chamada recursiva consegue reaplicar o
> próprio programa.

## 11. IR: CoreExpr

A IR é mínima:

```rust
enum CoreExpr {
    Nil,
    Atom(Bytes),
    Cons(Box<CoreExpr>, Box<CoreExpr>),
}
```

Em termos de Lisp:

- `Nil` é a lista vazia.
- `Atom` é um átomo de bytes.
- `Cons` é um par `(head . tail)`.

Por que isso é legal para compiladores?

- IR pequena.
- Fácil de serializar.
- Fácil de interpretar.
- Menos casos para a VM tratar.

Mensagem didática:

> A superfície tem várias formas; o núcleo tem só três. Essa é a função de uma
> boa IR: simplificar o restante do pipeline.

## 12. Opcodes

Operações são representadas por bytes.

Exemplos importantes:

| Operação | Nome | Ideia |
|---|---|---|
| `q` | quote | retorna dado sem avaliar |
| `a` | apply | executa programa em ambiente |
| `i` | if | escolhe branch |
| `x` | exception | aborta |
| `c` | cons | constrói par/lista |
| `+` | add | soma |
| `<` | lt | compara |
| `=` | eq | igualdade |

Você não precisa decorar bytes. Mas pode citar que `+` aparece como `0x17` no
core atual.

## 13. Codec / bytecode

Depois de gerar `CoreExpr`, o projeto serializa essa árvore para bytes.

Isso é a fase de codec:

```txt
CoreExpr -> encode -> bytes
bytes -> decode -> CoreExpr
```

Garantia importante:

```txt
decode(encode(expr)) == expr
```

Isso é testado com property tests.

Mensagem didática:

> O bytecode é a forma compacta e canônica do programa. O disassembler faz o
> caminho inverso para humanos conseguirem ler.

## 14. Disassembler

O disassembler transforma bytecode de volta em uma forma legível.

Exemplo da demo:

```txt
(+ (q . 0x28) (q . 0x02))
```

O que explicar:

- Não é exatamente o código fonte original.
- É a forma core/mnemonic do programa.
- Ajuda a visualizar o resultado do lowering + codec.

## 15. VM / evaluator

A VM executa `CoreExpr`.

Ela recebe:

1. O programa core.
2. Um ambiente.
3. Um contexto de avaliação com limites.

Ela retorna:

1. Um `Value`.
2. Ou um erro de avaliação.

Formas especiais:

- `q`: quote.
- `i`: if com short-circuit.
- `a`: apply.
- `x`: exception.

Demais opcodes são estritos:

> primeiro avalia argumentos, depois aplica operação.

Mensagem didática:

> A VM é um interpretador da IR, não do código fonte.

## 16. Value

`Value` é o tipo runtime.

Ele parece com `CoreExpr`, mas é usado na execução.

Conceitualmente:

- `Nil`.
- `Atom(bytes)`.
- `Cons(head, tail)`.

Exemplo:

```txt
0x2a
```

é o valor 42 em bytes little-endian.

## 17. Demo principal

Arquivo:

```txt
demo/presentation/01_add.btl
```

Conteúdo:

```lisp
(+ 40 2)
```

Comandos:

```sh
cargo run -p btclisp-cli -- check --core demo/presentation/01_add.btl
cargo run -p btclisp-cli -- compile demo/presentation/01_add.btl -o /tmp/btclisp-add.bin
cargo run -p btclisp-cli -- disasm /tmp/btclisp-add.bin
cargo run -p btclisp-cli -- run /tmp/btclisp-add.bin
```

Resultado esperado:

```txt
0x2a
```

Como explicar cada comando:

- `check --core`: valida sintaxe e mostra IR.
- `compile`: gera bytecode em arquivo.
- `disasm`: mostra bytecode como core legível.
- `run`: executa na VM.

## 18. Demo de recursão

Arquivo:

```txt
demo/presentation/02_factorial.btl
```

Conteúdo:

```lisp
(defun fact (n)
  (if (< n 2)
      1
      (* n (fact (- n 1)))))

(fact 5)
```

Resultado esperado:

```txt
0x78
```

`0x78` em decimal é 120.

Pontos para falar:

- Mostra função.
- Mostra condicional.
- Mostra recursão direta.
- Mostra que a IR gerada é maior que o fonte, porque o açúcar foi expandido.

## 19. Demo de `let` + `if`

Arquivo:

```txt
demo/presentation/03_let_if.btl
```

Conteúdo:

```lisp
(let ((x 40)
      (y 2))
  (if (= (+ x y) 42)
      1
      0))
```

Resultado esperado:

```txt
0x01
```

Pontos para falar:

- `let` cria ambiente.
- `if` seleciona branch.
- Variáveis desaparecem no core e viram caminhos.

## 20. Estratégia de testes

Tipos de teste no projeto:

### Unit tests

Testam partes pequenas:

- Lexer.
- Parser.
- Lowering.
- Codec.
- VM.

### Property tests

Especialmente no codec:

```txt
decode(encode(expr)) == expr
```

A ideia é gerar muitas árvores aleatórias e verificar se a propriedade sempre
vale.

### Golden tests

Os exemplos em `examples/` têm saídas esperadas em `.bin.hex` e `.expected`.

Isso garante que o compilador continue gerando os mesmos bytes para exemplos
conhecidos.

### End-to-end tests

Compilam programas e executam na VM.

Mensagem didática:

> O projeto testa tanto as peças isoladas quanto o pipeline inteiro.

## 21. Limitações importantes

Não apresente limitações como defeitos; apresente como decisões de escopo v0.1.

### Escopos não capturam variáveis externas

Exemplo que v0.1 não suporta bem:

```lisp
(defun f (a)
  (let ((b 1))
    (+ a b)))
```

Motivo: o ambiente v0.1 é flat e não-capturante.

### Recursão mútua não é suportada

Funciona:

```lisp
(defun fact (n) ... (fact (- n 1)))
```

Não funciona:

```lisp
(defun even (n) (odd (- n 1)))
(defun odd (n) (even (- n 1)))
```

### Inteiros são `u64`

A semântica de número é simples e intencional.

### Custos de execução são simples

A VM tem limites de operações e profundidade, mas não é um modelo de custo
consensus-grade.

## 22. Próximos passos do projeto

Se quiser mencionar futuro:

- Env-tree balanceada.
- Closures.
- Recursão mútua.
- Semântica de inteiros mais próxima de Bitcoin Script.
- Mais cobertura para sighash/opcodes Bitcoin.
- Modelo de custo mais sofisticado.

## 23. Perguntas prováveis e respostas curtas

### “Por que Lisp?”

Porque a sintaxe é simples, o parser fica pequeno e a estrutura de lista combina
bem com uma IR minimalista.

### “Isso compila para código de máquina?”

Não. Compila para uma IR/bytecode próprio e executa em uma VM interpretada.

### “Qual é a diferença entre AST e CoreExpr?”

A AST representa a linguagem que o usuário escreveu. `CoreExpr` representa uma
linguagem núcleo menor, sem a maior parte do açúcar sintático.

### “O que é lowering?”

É a transformação da AST de superfície para uma IR mais simples. Exemplos:
`let`, `if` e funções são reescritos usando formas menores do núcleo.

### “Por que serializar para bytes?”

Para ter uma representação canônica, compacta, comparável em testes e possível
de armazenar/transmitir.

### “A VM executa o fonte?”

Não. Ela executa `CoreExpr`, geralmente vindo do bytecode decodificado.

### “Onde entram os testes de compilador?”

Entram em todas as fases: lexer/parser/lowering/codec/VM e também no pipeline
completo.

### “Por que o resultado aparece como `0x2a` em vez de `42`?”

Porque valores runtime são átomos de bytes. `0x2a` é 42 em hexadecimal.

### “Por que o core parece maior que o código fonte?”

Porque açúcar sintático foi expandido. O core é mais explícito.

### “Qual parte é mais difícil?”

O lowering, porque ele precisa preservar a semântica do programa enquanto remove
construções de alto nível.

## 24. O que memorizar

Memorize estas frases:

1. “O projeto é um pipeline completo: texto, tokens, AST, IR, bytecode e VM.”
2. “A AST representa o que o usuário escreveu; a IR representa o que a VM entende.”
3. “Lowering é a fase que remove açúcar sintático.”
4. “Variáveis somem em runtime e viram caminhos no ambiente.”
5. “O bytecode é canônico e testado por round-trip.”
6. “A VM interpreta `CoreExpr`, não o código fonte.”
7. “Bitcoin é o domínio de aplicação, mas a apresentação é sobre compiladores.”

## 25. Checklist antes de apresentar

Na raiz do projeto:

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo build
```

Na pasta dos slides:

```sh
cd presentations/btclisp-compiladores
pnpm dev
```

Demo principal:

```sh
cargo run -p btclisp-cli -- check --core demo/presentation/01_add.btl
cargo run -p btclisp-cli -- compile demo/presentation/01_add.btl -o /tmp/btclisp-add.bin
cargo run -p btclisp-cli -- disasm /tmp/btclisp-add.bin
cargo run -p btclisp-cli -- run /tmp/btclisp-add.bin
```

Se algo der errado na demo, mostre os arquivos já prontos em
`demo/presentation/README.md` e explique o fluxo pelos outputs esperados.
