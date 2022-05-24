import path from "node:path";

function prepareFilenames(filenames) {
  const cwd = process.cwd();
  return filenames.map(file => path.relative(cwd, file)).join(" ");
}

export default {
  "*.rs": filenames => [
    `cargo fmt -- ${prepareFilenames(filenames)}`,
    // If this fails, try running
    //   cargo clippy --workspace --fix --allow-dirty --allow-staged
    "cargo clippy --workspace -- -D warnings",
  ],
  "*.{yaml,yml,md,js,ts,json}": filenames => [
    `prettier --write ${prepareFilenames(filenames)}`,
  ],
  "*.sh": filenames => [
    `shfmt -w ${prepareFilenames(filenames)}`,
    `shellcheck ${prepareFilenames(filenames)}`,
  ],
};
