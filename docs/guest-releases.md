# Reth stateless validator releases

The Reth guest accepts the execution-specs `statelessInputBytes` payload
unchanged and writes the corresponding `statelessOutputBytes`. It is released
for OpenVM, SP1, and ZisK under these names:

```text
stateless-validator-reth-openvm-v2.1.0-preview.elf
stateless-validator-reth-sp1-v6.4.0.elf
stateless-validator-reth-zisk-v1.1.0-alpha.elf
```

Each ELF has a verification key with the same basename and a `.vk` extension.
Releases also contain `SHA256SUMS` and GitHub artifact attestations.

## Build

The build scripts use digest-pinned ERE v0.16.2 compiler and server images:

```bash
scripts/guests/build.sh openvm output
scripts/guests/build.sh sp1 output
scripts/guests/build.sh zisk output
```

The three guest packages are independent Cargo workspaces with committed
lockfiles. A build fails if its lockfile changes or if the compiler does not
produce a non-empty ELF and verification key.

Manual workflow runs build and smoke-test all three artifacts without write
permissions. Pushing a `reth-guest-v*` tag runs the same matrix, creates
checksums, attests the artifacts, and creates a GitHub release after every
build and smoke test succeeds.

## Verify

After downloading a release, verify checksums and GitHub's signed provenance:

```bash
sha256sum --check SHA256SUMS
gh attestation verify stateless-validator-reth-sp1-v6.4.0.elf \
  --repo paradigmxyz/stateless
```

ERE consumers should download and republish these exact ELF and VK bytes rather
than rebuilding them in `ere-guests`.
