# The ditto code formatter

i.e. `ditto fmt`

Pretty prints ditto code. Heavily inspired by [`go fmt`](https://go.dev/blog/gofmt).

Shout-out to [`dprint`](https://github.com/dprint) for creating such a nice thing, and making this crate so much easier to implement 🙏

### Goals

- **Optimise for diffs.**. The code format should aim to make reviewing code changes as easy as possible.
- **Deterministic and idempotent**. A given input should always produce the same output, and running formatting _already formatted_ code should be a no-op.

### Non-goals:

- **Configurability.** I'll quote [`prettier`](https://prettier.io/docs/en/option-philosophy.html) here:

> debates over styles just turn into debates over which Prettier options to use.

### Obligatory Rob Pike quote

> Gofmt's style is no one's favorite, yet gofmt is everyone's favorite.
