// Keeps the version in package.json, tauri.conf.json and Cargo.toml the same.
//
//   node scripts/version.mjs            prints the version, failing if the three differ
//   node scripts/version.mjs 0.2.0      sets all three
//   node scripts/version.mjs --tag v0.2.0   fails unless the tag matches the version
import { readFileSync, writeFileSync } from "node:fs";

const files = {
  pkg: "package.json",
  conf: "src-tauri/tauri.conf.json",
  cargo: "src-tauri/Cargo.toml",
};
const cargoVersion = /^(version\s*=\s*")([^"]+)(")/m;

function read() {
  return {
    pkg: JSON.parse(readFileSync(files.pkg, "utf8")).version,
    conf: JSON.parse(readFileSync(files.conf, "utf8")).version,
    cargo: readFileSync(files.cargo, "utf8").match(cargoVersion)?.[2],
  };
}

function current() {
  const v = read();
  const all = new Set(Object.values(v));
  if (all.size !== 1) {
    console.error(`Versions differ: ${JSON.stringify(v)}. Run node scripts/version.mjs <version>.`);
    process.exit(1);
  }
  return v.pkg;
}

const [arg, value] = process.argv.slice(2);
if (!arg) {
  console.log(current());
} else if (arg === "--tag") {
  const version = current();
  if (value !== `v${version}`) {
    console.error(`Tag ${value} doesn't match the app version ${version}. Tag v${version}, or set the version first.`);
    process.exit(1);
  }
  console.log(`Tag ${value} matches.`);
} else {
  if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$/.test(arg)) {
    console.error(`${arg} isn't a version like 0.2.0.`);
    process.exit(1);
  }
  for (const f of [files.pkg, files.conf]) {
    const json = JSON.parse(readFileSync(f, "utf8"));
    json.version = arg;
    writeFileSync(f, JSON.stringify(json, null, 2) + "\n");
  }
  writeFileSync(files.cargo, readFileSync(files.cargo, "utf8").replace(cargoVersion, `$1${arg}$3`));
  console.log(`Version set to ${arg}. Commit, then tag v${arg} and push the tag.`);
}
