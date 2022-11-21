# The ditto command-line interface

<!-- prettier-ignore-start -->
```console
$ ditto --help
ditto 0.0.0
putting the fun in functional

USAGE:
    ditto <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    bootstrap    Bootstrap a new project
    make         Build a project
    fmt          Format ditto code
    lsp          Start up the language server

```
<!-- prettier-ignore-end -->

## `ditto bootstrap` - scaffold a new project ✨

<!-- prettier-ignore-start -->
```console
$ ditto bootstrap --help
ditto-bootstrap 
Bootstrap a new project

USAGE:
    ditto bootstrap [OPTIONS] <DIR>

ARGS:
    <DIR>    Directory for the project

OPTIONS:
    -h, --help           Print help information
        --js             JavaScript project?
        --name <name>    Optional package name (defaults to DIR)

```
<!-- prettier-ignore-end -->

## `ditto make` - build a project 👷

<!-- prettier-ignore-start -->
```console
$ ditto make --help
ditto-make 
Build a project

USAGE:
    ditto make [OPTIONS]

OPTIONS:
        --exec <execs>    Shell command to run on success
    -h, --help            Print help information
        --no-tests        Ignore test modules and dependencies
    -w, --watch           Watch files for changes

```
<!-- prettier-ignore-end -->

## `ditto fmt` - format ditto code 💅

<!-- prettier-ignore-start -->
```console
$ ditto fmt --help
ditto-fmt 
Format ditto code

USAGE:
    ditto fmt [OPTIONS] [globs]...

ARGS:
    <globs>...    

OPTIONS:
        --check    
    -h, --help     Print help information
        --stdin    

```
<!-- prettier-ignore-end -->

## `ditto lsp` - start the language server 🌐

<!-- prettier-ignore-start -->
```console
$ ditto lsp --help
ditto-lsp 
Start up the language server

USAGE:
    ditto lsp

OPTIONS:
    -h, --help    Print help information

```
<!-- prettier-ignore-end -->
