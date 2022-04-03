const path = require("node:path");
function prepareFilenames(filenames) {
  const cwd = process.cwd();
  return filenames.map(file => path.relative(cwd, file)).join(" ");
}
module.exports = {
  "*.rs": filenames => [
    `cargo fmt -- ${prepareFilenames(filenames)}`,
    "cargo clippy --workspace --fix --allow-dirty --allow-staged",
  ],
  "*.{yaml,yml,md,js,ts,json}": filenames => [
    `prettier --write ${prepareFilenames(filenames)}`,
  ],
  "*.sh": filenames => [
    `shfmt -w ${prepareFilenames(filenames)}`,
    `shellcheck ${prepareFilenames(filenames)}`,
  ],
};
