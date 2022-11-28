# The ditto command-line interface

<!-- prettier-ignore-start -->
```console
$ ditto --help
putting the fun in functional

Usage: ditto <COMMAND>

Commands:
  bootstrap  Bootstrap a new project
  make       Build a project
  fmt        Format ditto code
  lsp        Start up the language server

Options:
  -h, --help     Print help information
  -V, --version  Print version information

```
<!-- prettier-ignore-end -->

## `ditto bootstrap` - scaffold a new project ✨

<!-- prettier-ignore-start -->
```console
$ ditto bootstrap --help
Bootstrap a new project

Usage: ditto bootstrap [OPTIONS] <DIR>

Arguments:
  <DIR>  Directory for the project

Options:
      --js           JavaScript project?
      --name <NAME>  Optional package name (defaults to DIR)
  -h, --help         Print help information

```
<!-- prettier-ignore-end -->

## `ditto make` - build a project 👷

<!-- prettier-ignore-start -->
```console
$ ditto make --help
Build a project

Usage: ditto make [OPTIONS]

Options:
  -w, --watch       Watch files for changes
      --no-tests    Ignore test modules and dependencies
      --exec <CMD>  Shell command to run on success
  -h, --help        Print help information

```
<!-- prettier-ignore-end -->

## `ditto fmt` - format ditto code 💅

<!-- prettier-ignore-start -->
```console
$ ditto fmt --help
Format ditto code

Usage: ditto fmt [OPTIONS] [PATH]...

Arguments:
  [PATH]...  Files to format

Options:
      --check  Error if input(s) aren't formatted
      --stdin  Format stdin
  -h, --help   Print help information

```
<!-- prettier-ignore-end -->

## `ditto lsp` - start the language server 🌐

<!-- prettier-ignore-start -->
```console
$ ditto lsp --help
Start up the language server

Usage: ditto lsp

Options:
  -h, --help  Print help information

```
<!-- prettier-ignore-end -->
