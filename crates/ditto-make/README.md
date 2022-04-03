# The ditto build system 👷

This crate is responsible for building ditto projects.

It relies heavily on [`ninja`][ninja-build] for this &mdash; an idea shamelessly stolen from [ReScript](https://rescript-lang.org/docs/manual/latest/build-performance#under-the-hood).

There is _plenty_ of scope for optimisation here. Specifically, caching package-level `build.ninja` files and adding more asynchronous IO would deliver some big performance wins.

[ninja-build]: https://ninja-build.org/
