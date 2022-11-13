to get a cpu flamegraph install (once):
```bash
cargo install flamegraph
```

then to profile with a specific .lox input, e.g:
```bash
cargo flamegraph -- bench/fixtures/fib.lox
```
(might require a sudo)

then can inspect the flamegraph SVG in a browser:
```bash
open flamegraph.svg
```