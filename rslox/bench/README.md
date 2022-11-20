to get a cpu flamegraph install (once):
```bash
cargo install flamegraph
```

then to profile with a specific .lox input, e.g:
```bash
cargo flamegraph --root -- bench/fixtures/fib.lox
```

then can inspect the flamegraph SVG in a browser:
```bash
open flamegraph.svg
```