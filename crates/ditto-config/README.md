# The ditto config file 🔧

i.e. `ditto.toml`

```toml
# Name of this package.
#
# The package name must start with a lower case letter, and contain only
# lower case letters, numbers and hyphens ("-").
name = "my-thing"

# Direct dependencies.
#
# Packages mentioned here should exist in the package set.
dependencies = ["core", "js-task", "some-package"]

# Codegen targets.
# Defaults to `[]`, which implies that ditto code will only be type-checked.
# Available targets: web, nodejs
targets = ["web"]

# (Optional)
# Required ditto version.
#
# Syntax is inherited from Cargo, see:
# https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html
ditto-version = "^0.1"

# Add any additional packages/overrides here.
[package-set.packages]
some-package = { path = "../some-package" }
```
