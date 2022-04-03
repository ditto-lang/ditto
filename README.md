<h1 align="center">ditto</h1>

<p align="center">
  A small, pure functional language that targets other languages.
  <br><br>
  <a href="https://github.com/ditto-lang/ditto/actions/workflows/ci.yaml">
    <img src="https://github.com/ditto-lang/ditto/actions/workflows/ci.yaml/badge.svg?branch=main" alt="CI status">
  </a>
  <a href="https://github.com/ditto-lang/ditto/actions/workflows/security-audit.yaml">
    <img src="https://github.com/ditto-lang/ditto/actions/workflows/security-audit.yaml/badge.svg?branch=main" alt="Security audit status">
  </a>
  <a href="https://codecov.io/gh/ditto-lang/ditto">
    <img src="https://codecov.io/gh/ditto-lang/ditto/branch/main/graph/badge.svg?token=6TTRJO2KK0" alt="Code coverage badge"/>
  </a>
  <br><br>
  <img src="./images/hello-ditto.svg" alt="Hello world program in ditto">
  <br>
  <em>Syntax highlighting coming soon</em>
</p>

## Elevator pitch ⏱️

Ditto is a mashup of my favourite features from other languages.

It's functional, statically typed, and pure. But unlike many other languages in this category, it also aims to be ruthlessly simple - the syntax is small and the type system is low power.

It has no runtime of its own. There are far more mature compilers and ecosystems out there that I want to make use of, and would be naive to try and replace. So ditto can be thought of, instead, as an _alternative syntax_ for other tools.

Ditto is not just a compiler, it's _the_ language swiss army knife. Package management, linting, formatting, etc, are all handled by the `ditto` executable. Although this violates the [Unix philosophy](https://en.wikipedia.org/wiki/Unix_philosophy) it is my hope that it makes for a better developer experience. Because developer experience matters.

## Disclaimer ⚠️

Ditto is still pre-v0.1 and very unstable. It is only for the curious at this stage.

## Design notes

- "There should be one - and preferably only one - obvious way to do it." - [Zen of Python](https://peps.python.org/pep-0020/)
- "Small is beautiful" - [Lua design](https://web.stanford.edu/class/ee380/Abstracts/100310-slides.pdf)
- "Clear is better than clever" - [Go Proverb](https://go-proverbs.github.io/)
  - Alternatively, "Verbose is better than terse" - Me
  - Optimise for reading and understanding, not for writing.
- Best practice over backwards compatibility 🔥
- Anti-magic. Anti-fancy.
- Safe by default, but _explicit_ escape hatches when you need them.
- Embrace code generation.

[carbon-screenshot]: https://carbon.now.sh/?bg=rgba%28237%2C118%2C248%2C1%29&t=material&wt=none&l=text&width=581&ds=true&dsyoff=20px&dsblur=68px&wc=true&wa=false&pv=56px&ph=56px&ln=true&fl=1&fm=dm&fs=14px&lh=133%25&si=false&es=1x&wm=false&code=module%2520Hello.Ditto%2520exports%2520%28..%29%253B%250A%250Aimport%2520%28core%29%2520String%253B%250Aimport%2520%28node-readline%29%2520Readline%2520%28question%29%253B%250Aimport%2520%28js-console%29%2520Console%253B%250A%250Atype%2520Greeting%2520%253D%2520%250A%2520%2520%257C%2520Generic%250A%2520%2520%257C%2520Name%28String%29%253B%250A%250Agreeting_to_string%2520%253D%2520%28greeting%253A%2520Greeting%29%253A%2520String%2520-%253E%2520%250A%2509match%2520greeting%2520with%250A%2509%257C%2520Generic%2520-%253E%2520%2522Hello%2520there%21%2522%250A%2509%257C%2520Name%28name%29%2520-%253E%2520%2522Hello%2520there%252C%2520%2524%257Bname%257D%21%2522%253B%250A%250Amain%2520%253D%2520do%2520%257B%250A%2520%2520response%2520%253C-%2520question%28%2522What%27s%2520your%2520name%253F%2522%29%253B%250A%2520%2520let%2520greeting%2520%253D%2520%250A%2509if%2520String.is_empty%28response%29%2520then%2520%250A%2509%2509%2520Generic%250A%2509else%250A%2509%2509Name%28response%29%250A%250A%2520%2520greeting_to_string%28greeting%29%2520%257C%253E%2520Console.log%2509%250A%257D%253B
