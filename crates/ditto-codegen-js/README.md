# Ditto to JavaScript code generator.

This crate converts the ditto [AST](../ditto-ast) to JavaScript.

### Goals:

- **Look sensible and be reasonably performant.** The generated code doesn't have to look hand-written, but it should be readable.
- **Use the latest ECMAScript features where useful.** Transpiling to older ECMAScript versions is a one-way street. And there are a lot of reasons to prefer newer features where possible (e.g. for code size and performance reasons). So ditto starts with the latest features and leaves it to other tools to transpile/polyfill these for the target environment.
- **Be easily ["tree-shakeable"][tree shaking] by other tools.** This comes mostly from using ES6 `import` and `export` statements, which is linked to the previous goal.

### Non-goals:

- **Pretty printing.** The generated code will _not_ be read by a human most of the time, so there's no reason to waste computational effort pretty printing it.
- **Configurability.** Ditto will only target latest ECMAScript (see goals). Transpiling for older environments is best left to dedicated tools.

[tree shaking]: https://developer.mozilla.org/en-US/docs/Glossary/Tree_shaking
