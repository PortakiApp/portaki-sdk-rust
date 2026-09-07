<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://portaki.app/logo-dark.svg">
    <img src="https://portaki.app/logo-light.svg" width="177" height="48" alt="Portaki">
  </picture>
</p>

<h1 align="center">portaki-cli</h1>

<p align="center">
  <strong>Command-line toolchain for Portaki Wasm modules</strong><br>
  Binary name <code>portaki</code> — init, build, lint, test, and OCI publish.
</p>

<p align="center">
  <a href="https://crates.io/crates/portaki-cli"><img src="https://img.shields.io/crates/v/portaki-cli.svg" alt="crates.io"></a>
  <a href="https://docs.rs/portaki-cli"><img src="https://img.shields.io/docsrs/portaki-cli" alt="docs.rs"></a>
  <a href="https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License Apache-2.0"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.75+-dea584?logo=rust&logoColor=white" alt="Rust 1.75+"></a>
  <a href="https://extism.org/"><img src="https://img.shields.io/badge/Extism-Wasm-7C3AED" alt="Extism"></a>
  <a href="https://portaki.app"><img src="https://img.shields.io/badge/site-portaki.app-f59e0b" alt="portaki.app"></a>
</p>

<p align="center">
  <a href="#install">Install</a> ·
  <a href="#commands">Commands</a> ·
  <a href="#typical-workflow">Workflow</a> ·
  <a href="#related-crates">Crates</a> ·
  <a href="#license">License</a>
</p>

---

Authors write modules against [`portaki-sdk`](https://crates.io/crates/portaki-sdk). At build time this binary compiles to `wasm32`, merges proc-macro emissions from `OUT_DIR/portaki-emissions/`, and packages OCI layers for registries such as GHCR.

## Install

```bash
cargo install portaki-cli
# or tip of main:
cargo install --git https://github.com/PortakiApp/portaki-sdk --locked portaki-cli
```

Requires the Wasm target:

```bash
rustup target add wasm32-unknown-unknown
```

## Commands

| Command | Contract |
|---------|----------|
| `portaki init` | Scaffold a module from a template |
| `portaki build` | Compile Wasm + merge emissions → `manifest.json` |
| `portaki lint` | Validate capabilities, connectors, i18n keys |
| `portaki test` | Forward to `cargo test` in the module crate |
| `portaki publish` | Push the OCI artifact, then announce it to the registry |
| `portaki catalog` | Dump the SDUI primitive catalog |
| `portaki inspect` | Inspect a published OCI artifact |
| `portaki docs` / `dev` | Docs helper / local mock gateway (evolves with the SDK) |

## Typical workflow

```bash
cd modules/weather
portaki build --release
portaki lint
PORTAKI_PUBLISH_VERSION=0.3.5 portaki publish --registry ghcr.io/portakiapp
```

Image name: `ghcr.io/portakiapp/portaki-modules-<module-id>:<semver>`.

`publish` announces the version to the registry after the push (needs `portaki login`).
`--no-announce` skips it — the artifact then belongs to no catalogue.

Official modules: [`portaki-modules`](https://github.com/PortakiApp/portaki-modules).

## Related crates

| Crate | Role |
|-------|------|
| [`portaki-sdk`](https://crates.io/crates/portaki-sdk) | Host APIs + SDUI |
| [`portaki-sdk-macros`](https://crates.io/crates/portaki-sdk-macros) | Manifest emissions |
| [`portaki-connectors`](https://crates.io/crates/portaki-connectors) | Typed connector ops |
| [`portaki-test-utils`](https://crates.io/crates/portaki-test-utils) | Mock host for tests |

## License

[Apache-2.0](https://github.com/PortakiApp/portaki-sdk/blob/main/LICENSE) · Copyright 2026 Syntax Labs
