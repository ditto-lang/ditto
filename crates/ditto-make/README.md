# The ditto build system 👷

This crate is responsible for building ditto projects.

It relies heavily on [`ninja`][ninja-build] for this &mdash; an idea shamelessly stolen from [ReScript](https://rescript-lang.org/docs/manual/latest/build-performance#under-the-hood).

There is _plenty_ of scope for optimisation here. Specifically, caching package-level `build.ninja` files and adding more asynchronous IO would deliver some big performance wins.

[ninja-build]: https://ninja-build.org/

## Internal CLI

<!-- prettier-ignore-start -->
```console
$ ditto compile --help
Usage: ditto compile [COMMAND]

Commands:
  ast           
  js            
  package-json  

Options:
  -h, --help  Print help information

```
<!-- prettier-ignore-end -->

<!-- prettier-ignore-start -->
```console
$ ditto compile ast --help
Usage: ditto compile ast --build-dir <DIR> -i <inputs>... -o <outputs>...

Options:
      --build-dir <DIR>  
  -i <inputs>...         
  -o <outputs>...        
  -h, --help             Print help information

```
<!-- prettier-ignore-end -->

<!-- prettier-ignore-start -->
```console
$ ditto compile js --help
Usage: ditto compile js -i <inputs>... -o <outputs>...

Options:
  -i <inputs>...       
  -o <outputs>...      
  -h, --help           Print help information

```
<!-- prettier-ignore-end -->

<!-- prettier-ignore-start -->
```console
$ ditto compile package-json --help
Usage: ditto compile package-json -i <input> -o <output>

Options:
  -i <input>       
  -o <output>      
  -h, --help       Print help information

```
<!-- prettier-ignore-end -->
