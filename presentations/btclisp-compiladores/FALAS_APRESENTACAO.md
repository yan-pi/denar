# Exemplos de falas para a apresentação

Este arquivo contém sugestões de fala para uma apresentação de aproximadamente
25 minutos. Use como roteiro, não como texto para ler palavra por palavra.

## Tom da apresentação

Tente manter este tom:

- Claro.
- Didático.
- Focado em compiladores.
- Sem aprofundar Rust/Bitcoin além do necessário.

Frase de posicionamento:

> “O projeto tem uma motivação ligada a Bitcoin, mas hoje eu vou usar ele como
> estudo de caso de compilador: como código fonte vira uma representação interna,
> depois bytecode, e finalmente resultado em uma VM.”

## Slide 1 — Título

Tempo sugerido: 1 minuto.

Fala possível:

> “Hoje eu vou apresentar o btclisp, que é uma linguagem pequena estilo Lisp e
> um compilador completo para ela. A ideia da apresentação não é defender Lisp,
> Rust ou Bitcoin, mas usar esse projeto como um exemplo concreto de pipeline de
> compilação. A gente vai sair de um arquivo de texto, passar por lexer, parser,
> AST, lowering, IR, bytecode e terminar executando isso em uma VM.”

Alternativa mais curta:

> “A pergunta que guia a apresentação é: o que precisa acontecer para um código
> como `(+ 40 2)` virar algo que uma máquina virtual consegue executar?”

## Slide 2 — Ideia central / pipeline

Tempo sugerido: 2 minutos.

Fala possível:

> “Esse é o pipeline inteiro. Primeiro temos o `.btl`, que é o arquivo fonte da
> linguagem. O lexer transforma texto em tokens. O parser transforma tokens em
> uma árvore sintática. Depois vem o lowering, que simplifica a linguagem para
> uma representação intermediária chamada `CoreExpr`. Essa IR é serializada para
> bytes, e a VM interpreta o programa e devolve um valor.”

Ênfase:

> “O ponto importante é que cada etapa tem uma responsabilidade bem separada. O
> lexer não sabe executar; o parser não sabe serializar; a VM não sabe ler o
> código fonte original.”

Se quiser conectar com aula:

> “Isso se encaixa bem no que a gente vê em compiladores: frontend, representação
> intermediária e backend/runtime.”

## Slide 3 — O que é btclisp?

Tempo sugerido: 1 minuto.

Fala possível:

> “btclisp é uma linguagem pequena com sintaxe Lisp. Isso significa que a
> estrutura do programa é escrita com parênteses e listas. Um exemplo mínimo é
> `(+ 40 2)`. O compilador pega isso, gera uma IR, serializa para bytes e depois
> a VM executa. O resultado aparece como `0x2a`, que é 42 em hexadecimal.”

Complemento:

> “A escolha por Lisp ajuda bastante didaticamente, porque a gramática é simples
> e a própria sintaxe já parece uma árvore.”

## Slide 4 — O que vamos tratar como black box

Tempo sugerido: 1 minuto.

Fala possível:

> “O projeto real tem ligação com Bitcoin, principalmente porque alguns opcodes
> lidam com assinatura, hash e dados de transação. Mas para esta apresentação eu
> vou tratar isso como domínio de aplicação. A parte que interessa aqui é a
> estrutura de compilador. Então Rust é a linguagem de implementação e Bitcoin é
> um contexto, mas o nosso foco é o pipeline.”

Se alguém parecer perdido:

> “Dá para entender a apresentação inteira pensando só em uma linguagem pequena
> que compila para uma VM própria.”

## Slide 5 — Linguagem fonte

Tempo sugerido: 2 minutos.

Fala possível:

> “Aqui estão algumas construções que o usuário pode escrever. A primeira é só
> uma soma. A segunda usa `let`, que cria variáveis locais. A terceira parece uma
> função recursiva comum: se `n` for menor que 2, retorna 1; caso contrário,
> multiplica `n` por `fact(n - 1)`.”

Ponto didático:

> “Essas construções são amigáveis para quem escreve o programa, mas a VM não
> precisa entender todas elas diretamente. Muitas vão ser reescritas pelo
> compilador.”

Frase boa:

> “A linguagem de superfície é para humanos; a linguagem núcleo é para a VM.”

## Slide 6 — Lexer

Tempo sugerido: 2 minutos.

Fala possível:

> “A primeira etapa é o lexer. Ele pega uma string de caracteres e transforma em
> uma sequência de tokens. Por exemplo, `(+ 40 2)` vira abre parêntese, símbolo
> `+`, inteiro 40, inteiro 2 e fecha parêntese.”

Explicação de spans:

> “Cada token carrega um `span`, que é a posição no arquivo. Isso é importante
> para mostrar mensagens de erro boas. Se o usuário escreve um literal inválido,
> o compilador consegue apontar onde está o problema.”

Analogia:

> “O lexer não entende ainda a estrutura do programa. Ele só separa e classifica
> pedaços do texto.”

## Slide 7 — Parser

Tempo sugerido: 2 minutos.

Fala possível:

> “Depois vem o parser. Ele pega os tokens e monta uma árvore. Como a linguagem é
> Lisp, o parser é recursivo-descendente e relativamente simples. Quando ele vê
> um parêntese abrindo, ele sabe que está entrando em uma lista ou aplicação.”

Exemplo:

> “A expressão `(+ 40 2)` vira uma aplicação cujo `head` é `+` e os argumentos
> são 40 e 2. Isso já é muito melhor do que texto puro, porque agora o programa
> tem estrutura.”

Frase boa:

> “Depois do parser, o compilador não precisa mais pensar em caracteres; ele
> pensa em nós de uma árvore.”

## Slide 8 — AST de superfície

Tempo sugerido: 2 minutos.

Fala possível:

> “Essa AST ainda representa a linguagem que o usuário escreveu. Por isso ela
> tem nós para `let`, `if`, `cond`, `quote`, aplicações e definições de função.
> Ela é uma representação de alto nível.”

Ponto central:

> “Mas a VM não executa diretamente essa AST. Ela seria grande demais e cheia de
> casos especiais. Então a próxima etapa é transformar essa AST em uma IR menor.”

Frase boa:

> “A AST preserva a intenção do programador; a IR preserva o que o runtime
> realmente precisa executar.”

## Slide 9 — Lowering

Tempo sugerido: 3 minutos.

Fala possível:

> “Lowering é a fase em que a linguagem de superfície é reescrita para uma
> linguagem núcleo. Essa é provavelmente a fase mais interessante do projeto do
> ponto de vista de compiladores.”

Explique exemplos:

> “Um `let` não precisa existir na VM. Ele pode virar uma aplicação de um corpo
> em um novo ambiente. Um `if` também é reescrito de forma que só o branch
> escolhido seja avaliado. Chamadas de função são transformadas para aplicações
> mais primitivas.”

Mensagem importante:

> “Lowering não é só trocar sintaxe. Ele precisa preservar semântica. Se o `if`
> avaliasse os dois branches, o programa poderia mudar de comportamento.”

## Slide 10 — CoreExpr

Tempo sugerido: 2 minutos.

Fala possível:

> “A representação intermediária do projeto se chama `CoreExpr`. Ela é bem
> pequena: só tem `Nil`, `Atom` e `Cons`. Isso é bem inspirado em Lisp: tudo pode
> ser representado como átomos e pares.”

Ponto didático:

> “A vantagem é que as fases depois do lowering ficam muito mais simples. O
> codec só precisa serializar três tipos de nó, e a VM só precisa interpretar
> essa estrutura mínima.”

Frase boa:

> “A superfície tem várias formas; o núcleo tem só três.”

## Slide 11 — Variáveis e ambiente

Tempo sugerido: 2 minutos.

Fala possível:

> “Um detalhe interessante é que nomes de variáveis não chegam até a VM. Se o
> usuário escreve `(+ a b)`, o compilador resolve `a` e `b` para caminhos no
> ambiente. Isso é parecido com transformar nomes em offsets.”

Exemplo:

> “No ambiente flat da versão atual, o primeiro parâmetro fica em um caminho, o
> segundo em outro, e assim por diante. Então a VM só precisa seguir o caminho no
> ambiente, não procurar uma string chamada `a`.”

Frase boa:

> “Nome é algo do frontend. Runtime gosta de posição.”

## Slide 12 — Bytecode e codec

Tempo sugerido: 2 minutos.

Fala possível:

> “Depois de gerar `CoreExpr`, o projeto serializa essa árvore para bytes. Essa
> etapa é o codec. O objetivo é ter uma representação canônica: para uma mesma
> expressão, existe uma forma de bytes esperada.”

Property tests:

> “Uma propriedade importante é `decode(encode(expr)) == expr`. O projeto testa
> isso com property tests, gerando muitas expressões aleatórias e verificando se
> o round-trip preserva a árvore.”

Frase boa:

> “O bytecode é para a máquina; o disassembler é para a gente conseguir conferir
> o que foi gerado.”

## Slide 13 — VM eager

Tempo sugerido: 2 minutos.

Fala possível:

> “A VM interpreta `CoreExpr`. Ela é eager: em geral, avalia os argumentos antes
> de aplicar a operação. Mas algumas formas são especiais: `q`, que é quote; `i`,
> que é if; `a`, que aplica um programa a um ambiente; e `x`, que gera exceção.”

Explicação simples:

> “Para `(+ 40 2)`, os dois argumentos são avaliados, a operação de soma é
> aplicada e o resultado é um `Value`, que no terminal aparece como `0x2a`.”

## Slide 14 — Demo ao vivo

Tempo sugerido: 4 minutos.

Fala antes de começar:

> “Agora vou mostrar o pipeline funcionando com um programa escrito do zero.”

Passo 1:

```sh
cargo run -p btclisp-cli -- check --core demo/presentation/01_add.btl
```

Fala:

> “O `check` valida o arquivo e, com `--core`, mostra a IR gerada. Aqui já dá
> para ver que o código fonte foi reescrito para a forma núcleo.”

Passo 2:

```sh
cargo run -p btclisp-cli -- compile demo/presentation/01_add.btl -o /tmp/btclisp-add.bin
```

Fala:

> “Agora eu gero o bytecode em um arquivo binário.”

Passo 3:

```sh
cargo run -p btclisp-cli -- disasm /tmp/btclisp-add.bin
```

Fala:

> “O disassembler lê os bytes e mostra uma forma legível do programa core. Não é
> exatamente o fonte original; é o programa depois do lowering.”

Passo 4:

```sh
cargo run -p btclisp-cli -- run /tmp/btclisp-add.bin
```

Fala:

> “Por fim, a VM executa o bytecode. O resultado é `0x2a`, que é 42.”

Fechamento da demo:

> “Então neste exemplo pequeno a gente percorreu o pipeline inteiro: fonte,
> core, bytes, disassembly e execução.”

## Slide 15 — Demo extra: recursão

Tempo sugerido: 2 minutos, se sobrar tempo.

Fala possível:

> “Se eu usar o exemplo do factorial, o fonte ainda parece bem simples, mas o
> core gerado fica maior. Isso é esperado: o compilador expandiu função,
> condicional e recursão para operações mais primitivas.”

Comandos:

```sh
cargo run -p btclisp-cli -- check --core demo/presentation/02_factorial.btl
cargo run -p btclisp-cli -- compile demo/presentation/02_factorial.btl -o /tmp/btclisp-fact.bin
cargo run -p btclisp-cli -- run /tmp/btclisp-fact.bin
```

Fala para resultado:

> “O resultado é `0x78`, que em decimal é 120, o valor de 5 fatorial.”

## Slide 16 — Estratégia de testes

Tempo sugerido: 2 minutos.

Fala possível:

> “Uma parte importante do projeto é que as fases são testadas separadamente e
> também em conjunto. Existem testes unitários para lexer, parser, lowering,
> codec e VM. Existem property tests para o codec, principalmente para garantir o
> round-trip entre árvore e bytes. E existem golden tests, que comparam programas
> de exemplo com bytes esperados.”

Frase boa:

> “Isso é importante em compiladores porque uma mudança pequena no lowering ou
> no encoding pode alterar a semântica ou o bytecode gerado.”

Comandos se quiser mostrar:

```sh
cargo test --workspace
```

## Slide 17 — Limitações v0.1

Tempo sugerido: 2 minutos.

Fala possível:

> “O projeto tem algumas limitações intencionais na versão atual. A principal é
> que o ambiente é flat e não captura variáveis externas. Então closures de
> verdade ainda não são suportadas. Recursão direta funciona, mas recursão mútua
> não. E a semântica de inteiros é simplificada.”

Como apresentar sem parecer problema:

> “Eu vejo essas limitações como decisões de escopo. Elas deixam a v0.1 pequena o
> suficiente para entender, e também indicam os próximos passos naturais do
> compilador.”

## Slide 18 — Conclusão

Tempo sugerido: 1 minuto.

Fala possível:

> “Para concluir: o btclisp é interessante porque mostra o ciclo completo de um
> compilador em uma escala pequena. Ele começa com texto, passa por tokens, AST,
> IR, bytecode e termina em uma VM. A parte de Bitcoin dá um domínio real, mas o
> aprendizado principal é sobre separação de fases, representação intermediária
> e preservação de semântica durante o lowering.”

Fechamento:

> “A pergunta que fica é: se eu quisesse transformar isso em uma linguagem maior,
> qual fase eu teria que evoluir primeiro? Provavelmente o modelo de ambiente,
> para suportar closures e recursão mútua.”

## Fala caso a demo falhe

Use sem hesitar:

> “Como demo ao vivo sempre pode depender do ambiente, eu deixei os comandos e
> os resultados esperados documentados. O fluxo esperado é: `check --core` mostra
> a IR, `compile` gera o bytecode, `disasm` mostra o core legível e `run` retorna
> `0x2a`. Esse fluxo já foi validado antes da apresentação.”

Depois mostre:

```txt
demo/presentation/README.md
```

## Fala para perguntas difíceis

### Se perguntarem: “Por que não gerar código de máquina?”

> “Porque o objetivo do projeto é ter uma VM própria e uma representação compacta
> para esse domínio. Gerar código de máquina seria outro backend. Aqui o backend
> é o bytecode + VM.”

### Se perguntarem: “Isso é interpretador ou compilador?”

> “Tem os dois. Existe uma etapa de compilação do fonte para IR/bytecode, e
> existe uma VM que interpreta esse bytecode.”

### Se perguntarem: “Onde está a análise semântica?”

> “Ela aparece principalmente na resolução de nomes, checagem de aridade,
> detecção de identificadores indefinidos e validação de formas que a v0.1 não
> suporta.”

### Se perguntarem: “O que acontece com variáveis?”

> “Variáveis são resolvidas no frontend. Depois do lowering elas viram caminhos
> no ambiente. A VM não procura nomes por string.”

### Se perguntarem: “Por que `if` precisa ser especial?”

> “Porque em uma VM eager, se `if` fosse uma operação comum, os dois branches
> seriam avaliados antes da escolha. Isso mudaria a semântica. Por isso o
> lowering e a VM tratam `if` de modo especial, com branches quotados e
> short-circuit.”

### Se perguntarem: “Qual foi a parte mais importante?”

> “Para mim, o lowering. É nele que a linguagem amigável é transformada em uma
> linguagem núcleo pequena, preservando o comportamento do programa.”

### Se perguntarem: “Qual seria a próxima melhoria?”

> “O modelo de ambiente. Hoje ele é flat e não-capturante. A evolução natural é
> uma env-tree com closures, o que permitiria escopos aninhados mais completos e
> possivelmente recursão mútua.”

## Roteiro compacto para memorizar

Se precisar decorar só uma sequência, decore esta:

1. “O projeto é um mini-compilador completo.”
2. “Fonte `.btl` vira tokens pelo lexer.”
3. “Tokens viram AST pelo parser.”
4. “A AST ainda tem açúcar sintático.”
5. “Lowering remove esse açúcar e gera `CoreExpr`.”
6. “`CoreExpr` é uma IR mínima: nil, atom e cons.”
7. “O codec transforma essa IR em bytes canônicos.”
8. “A VM interpreta esses bytes/CoreExpr e produz um `Value`.”
9. “A demo mostra exatamente esse caminho.”
10. “As limitações atuais apontam para closures/env-tree como próximo passo.”

## Roteiro de 25 minutos

Sugestão de tempo:

| Parte | Tempo |
|---|---:|
| Abertura e motivação | 2 min |
| Pipeline geral | 3 min |
| Linguagem fonte | 3 min |
| Lexer/parser/AST | 5 min |
| Lowering/CoreExpr | 6 min |
| Bytecode/VM | 3 min |
| Demo | 4 min |
| Testes/limitações/conclusão | 2 min |

Se estiver estourando tempo, corte detalhes de Bitcoin, codec e recursão.

Se estiver sobrando tempo, mostre o factorial.
