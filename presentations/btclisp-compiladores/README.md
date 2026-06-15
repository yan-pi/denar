# Slides — btclisp para compiladores

Deck em Slidev para uma apresentação de aproximadamente 25 minutos.

## Instalar

```sh
pnpm install
```

## Rodar em modo apresentação

```sh
pnpm dev
```

## Build estático

```sh
pnpm build
```

## Exportar

```sh
pnpm export
```

Se o export pedir Playwright/Chromium, rode:

```sh
pnpm exec playwright install chromium
```

## Roteiro

O deck trata Rust e Bitcoin como contexto. O foco é o pipeline de compilador:
léxico, parser, AST, lowering, IR, bytecode, VM e testes.
