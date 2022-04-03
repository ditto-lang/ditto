import fs from "node:fs";
import crypto from "node:crypto";
import path from "node:path";
import cp from "node:child_process";
import assert from "node:assert";

import arg from "arg";
import archiver from "archiver";

function spawn(command, args) {
  return new Promise((resolve, reject) => {
    console.log(`$ ${command} ${args.join(" ")}`);
    const child = cp.spawn(command, args);
    child.stdout.on("data", function (data) {
      process.stdout.write(data);
    });
    child.stderr.on("data", function (data) {
      process.stderr.write(data);
    });
    child.on("error", function (err) {
      reject(err);
    });
    child.on("exit", function (code) {
      resolve(code);
    });
  });
}

function sha256sum(file) {
  const buffer = fs.readFileSync(file);
  const hashSum = crypto.createHash("sha256");
  hashSum.update(buffer);
  return hashSum.digest("hex");
}

function zip(files, outPath) {
  console.log(`==> Creating zip archive: ${outPath}`);

  return new Promise((resolve, reject) => {
    const output = fs.createWriteStream(outPath);
    const archive = archiver("zip", { zlib: { level: 9 } });

    output.on("close", () => {
      console.log(archive.pointer() + " total bytes written");
      resolve();
    });
    archive.on("warning", err => {
      if (err.code === "ENOENT") {
        console.log(`WARNING: ${err}`);
      } else {
        reject(err);
      }
    });
    archive.on("err", err => {
      reject(err);
    });
    for (const file of files) {
      archive.file(file, { name: path.basename(file) });
    }

    archive.pipe(output);
    archive.finalize();
  });
}
async function checkDittoBin(ditto) {
  console.log(`==> Checking ditto binary: ${ditto}`);
  assert.equal(await spawn(ditto, ["--version"]), 0, "ditto --version works");
  assert.equal(
    await spawn(ditto, ["bootstrap", "bootstrap-test"]),
    0,
    "ditto bootstrap works",
  );
}

async function main() {
  const {
    "--ditto-bin": dittoBin,
    "--out-zip": outZip,
    "--out-sha256": outSha256,
  } = arg({
    "--ditto-bin": String,
    "--out-zip": String,
    "--out-sha256": String,
  });
  if (!dittoBin) {
    throw new Error("missing required argument: --ditto-bin");
  }
  if (!outZip) {
    throw new Error("missing required argument: --out-zip");
  }
  if (!outSha256) {
    throw new Error("missing required argument: --out-sha256");
  }

  await checkDittoBin(dittoBin);
  await zip([dittoBin], outZip);

  console.log(`==> Generating sha256 for ${outZip}`);
  const sha256 = sha256sum(outZip);
  console.log(sha256);
  fs.writeFileSync(outSha256, sha256);
  console.log(`${outSha256} written`);
}

main();
