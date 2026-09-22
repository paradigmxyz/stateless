// Disposable fixtures only; never run on the PR branch or publish these packages.
const fs = require('node:fs');
const path = require('node:path');
const [mode, guest, dependency, version] = process.argv.slice(2);
const dir = path.join('bin/stateless-validator-reth', guest);
const manifest = path.join(dir, 'Cargo.toml');
if (mode === 'stale-lock') {
  const probe = path.join(dir, 'lockfile-probe');
  fs.mkdirSync(path.join(probe, 'src'), { recursive: true });
  fs.writeFileSync(path.join(probe, 'Cargo.toml'), '[package]\nname = "lockfile-probe"\nversion = "0.0.0"\nedition = "2021"\n');
  fs.writeFileSync(path.join(probe, 'src/lib.rs'), '#![no_std]\npub fn probe() {}\n');
  fs.appendFileSync(manifest, '\n[dependencies.lockfile-probe]\npath = "lockfile-probe"\n');
  // Deliberately leave the existing Cargo.lock untouched.
} else if (mode === 'cooldown') {
  fs.writeFileSync(manifest, `[workspace]\n[package]\nname = "guest-gate-fixture"\nversion = "0.0.0"\nedition = "2021"\n[dependencies]\n${dependency} = "=${version}"\n`);
  fs.writeFileSync(path.join(dir, 'src/main.rs'), 'fn main() {}\n');
  fs.rmSync(path.join(dir, 'Cargo.lock'));
} else {
  throw new Error('Unknown fixture mode');
}
