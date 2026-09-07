<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://portaki.app/logo-dark.svg">
    <img src="https://portaki.app/logo-light.svg" width="177" height="48" alt="Portaki">
  </picture>
</p>

<h1 align="center">portaki-sdk</h1>

<p align="center">
  <strong>Rust SDK, CLI, connectors, and test utilities for Portaki Wasm guest modules</strong><br>
  Build, lint, test, and publish Extism modules as OCI images on GitHub Container Registry.
</p>

<p align="center">
  <a href="https://github.com/PortakiApp/portaki-sdk/actions/workflows/ci.yml"><img src="https://github.com/PortakiApp/portaki-sdk/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License Apache-2.0"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.75+-dea584?logo=rust&logoColor=white" alt="Rust 1.75+"></a>
  <a href="https://extism.org/"><img src="https://img.shields.io/badge/Extism-Wasm-7C3AED" alt="Extism"></a>
  <a href="https://portaki.app"><img src="https://img.shields.io/badge/site-portaki.app-f59e0b" alt="portaki.app"></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#publish-oci--ghcr">Publish</a> ·
  <a href="#workspace-crates">Crates</a> ·
  <a href="docs/connectors-and-credentials.md">Connectors & credentials</a> ·
  <a href="docs/module-layout.md">Module layout</a> ·
  <a href="docs/typed-ids.md">Typed ids</a> ·
  <a href="CONTRIBUTING.md">Contributing</a> ·
  <a href="SECURITY.md">Security</a>
</p>

---

Portaki runs guest modules as **Extism Wasm** plugins. This workspace is what module authors use day to day: host APIs, proc-macros, connectors, mocks, and the `portaki` CLI.

Official modules are published from [`portaki-modules`](https://github.com/PortakiApp/portaki-modules) as:

`ghcr.io/portakiapp/portaki-modules-<module-id>:<semver>`

## Why this SDK?

- **One toolchain** — `portaki init` / `build` / `lint` / `test` / `publish`
- **Compile-time metadata** — macros emit catalog + SDK manifests consumed by the host
- **Connectors** — typed clients for OpenWeather, Google Places, Mapbox, OSM, …
- **Testable** — `MockContext` and in-memory host functions without a full runtime
- **OCI-native** — push to GHCR with dash-named packages (listed by the GitHub Packages API)

## Connectors and credentials

Declare pool + BYOK with `#[custom_connector(... credential_provider_id = "...")]` and `external.<provider>.pool` / `.byok` capabilities. The orchestrator derives which module needs which key from the published manifest — no dashboard hardcoding.

See **[docs/connectors-and-credentials.md](./docs/connectors-and-credentials.md)** (weather is the reference module).

## Workspace crates

| Crate | Purpose |
|-------|---------|
| [`portaki-sdk`](./crates/portaki-sdk) | Host function wrappers, SDUI catalog, capability constants |
| [`portaki-sdk-macros`](./crates/portaki-sdk-macros) | Proc-macros that emit manifest metadata at compile time |
| [`portaki-connectors`](./crates/portaki-connectors) | Typed external connectors |
| [`portaki-test-utils`](./crates/portaki-test-utils) | `MockContext`, in-memory host functions, SDUI assertions |
| [`portaki-cli`](./crates/portaki-cli) | `portaki` binary |

## Requirements

- Rust **1.75+**
- Target `wasm32-unknown-unknown` for module builds

```bash
rustup target add wasm32-unknown-unknown
```

## Install

```bash
cargo install --git https://github.com/PortakiApp/portaki-sdk --branch main --locked portaki-cli
```

## Quick start

```bash
cargo build --workspace
cargo test --workspace

cargo run -p portaki-cli -- init my-module --template default
cd my-module
portaki build --release
portaki lint
```

## Publish (OCI / GHCR)

After `portaki build --release`:

```bash
export GITHUB_TOKEN="<classic-pat-with-write:packages>"   # or: docker login ghcr.io
export PORTAKI_PUBLISH_VERSION="0.2.1"                    # optional CI guard
portaki publish --registry ghcr.io/portakiapp
```

Image name: `ghcr.io/portakiapp/portaki-modules-<module-id>:<semver>` (dash, not slash).

`portaki publish --dry-run` validates the artifact without pushing.

### The registry announcement

Pushing to GHCR puts the bytes somewhere; it does not put the module in a catalogue. After the
push, `publish` announces the version to the Portaki registry — module id, version, artifact ref
and the digest read back from GHCR — which pulls the artifact by that digest and reads the
manifest inside it. The body says *where to look*, never *what was published*.

That announcement needs `portaki login`. Replaying a version that is already published is not an
error: publications are immutable and keyed by digest, so a re-run of a CI job is a no-op.

`--no-announce` pushes to GHCR only. The artifact then exists in no catalogue — useful to inspect
a build, not to ship one.

When both `portaki.module.json` and SDK emissions exist, publish pushes catalog + SDK layers
(plus optional migrations / operations / i18n / wasm):

- `application/vnd.portaki.manifest+json` — host catalog (`publish-manifest.json`)
- `application/vnd.portaki.sdk.manifest+json` — SDK emissions (`manifest.json`)
- `application/vnd.portaki.migrations+json` — `migrations.bundle.json` (when `db/migrations/` exists)
- `application/vnd.portaki.operations+json` — `operations.bundle.json` v2 schema from `#[entity]` (typed-repo upsert)

## Related repositories

| Repository | Role |
|------------|------|
| [portaki-modules](https://github.com/PortakiApp/portaki-modules) | Official Wasm modules monorepo |
| [portaki.app](https://portaki.app) | Product site |

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) and the [Code of Conduct](./CODE_OF_CONDUCT.md).

Security issues: [SECURITY.md](./SECURITY.md) — do not open a public issue.

## License

[Apache-2.0](./LICENSE) · Copyright 2026 Syntax Labs
