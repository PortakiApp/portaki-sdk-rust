# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.2.0](https://github.com/PortakiApp/portaki-sdk/compare/v2.1.1...v2.2.0) (2026-09-07)


### Features

* **cli:** annoncer la publication au registre ([a7715d8](https://github.com/PortakiApp/portaki-sdk/commit/a7715d80783b223aacef99bdd9f28621c2c6cb0b))
* **cli:** annoncer une version deja sur GHCR ([90cd7b8](https://github.com/PortakiApp/portaki-sdk/commit/90cd7b8d2598fee4f8750fd6be6511b9bff071d5))
* **cli:** portaki dev builds, deploys and shows the run ([ec4cdce](https://github.com/PortakiApp/portaki-sdk/commit/ec4cdce2de0a71e1870ee9d44e2ebdd93c140eaa))
* **cli:** portaki login stores the token in the system keychain ([28c83aa](https://github.com/PortakiApp/portaki-sdk/commit/28c83aaeefa4f145bbdf599828ecf3d204062344))
* **cli:** tamponner la version SDK liee au build ([08ec4bd](https://github.com/PortakiApp/portaki-sdk/commit/08ec4bd95ca8454ef6df99594a742b717fba2747))
* **connectors:** add OpenAgenda nearby events client ([ab53d1f](https://github.com/PortakiApp/portaki-sdk/commit/ab53d1f9c2a1f66170905c86a012f1fd47fcd123))
* **context:** expose stay booking_channel to modules ([3ac7d48](https://github.com/PortakiApp/portaki-sdk/commit/3ac7d4816740a87da607eaec0dbcd38f42c3fc78))
* **contracts:** add booking channel vocabulary ([6e84543](https://github.com/PortakiApp/portaki-sdk/commit/6e845434dc761e912bb0ba747f05eaa245495da2))
* **contracts:** add shared StayImportRow shape ([c695a53](https://github.com/PortakiApp/portaki-sdk/commit/c695a539749830d7785e9328f6d7ac1bfe86c4f8))
* **host:** add host::notify + core.host.notifications capability ([a3b8715](https://github.com/PortakiApp/portaki-sdk/commit/a3b87157cc14acde7553892b2049589f9045743e))
* **schema:** add maturity and sortOrder fields ([411e626](https://github.com/PortakiApp/portaki-sdk/commit/411e62640a08cb2c6b2567ffed41b2bfbaa7eb87))
* **schema:** declare module permissions, rename SDK field ([f588b92](https://github.com/PortakiApp/portaki-sdk/commit/f588b92010e005dc2edf527ae8cc4c102cfee82b))
* **sdui:** add optional blurHash to Image ([5c4ce59](https://github.com/PortakiApp/portaki-sdk/commit/5c4ce598137e840390413ab710c95cec36717b0b))


### Bug Fixes

* **cli:** read the platform envelope, renew on 401 ([f00bbcc](https://github.com/PortakiApp/portaki-sdk/commit/f00bbcc334c196cf58cc78a3dbb7cdc54ae8dd20))
* **deps:** update rust crate extism-pdk to 1.4.1 ([440ca31](https://github.com/PortakiApp/portaki-sdk/commit/440ca31322af04242dc0fe13e2bf290b7cb9004e))

## [Unreleased]

### Features

* **contracts:** add `booking_channel` — canonical `BookingChannel` /
  `ChannelSignal` vocabulary answering *who sold the stay* (vocabulary only, no
  decision table; behavioural attributes stay on the gateway)
* **contracts:** add `stay_import::StayImportRow` — canonical import row shape
  for `ModuleGatewayStayImportAdapter`, now carrying `bookingChannel` /
  `bookingChannelSignal` on every row
* **sdui:** add optional `icon` on `ToggleRow` (leading icon token)
* **sdui:** add `IndexedInput` (index + optional checkbox + text field tile)
* **sdui:** add `Grid.minColumnWidth` for auto-fit host grids

## [2.1.1] — 2026-07-25

### Features

* **email:** `LocalizedEmailText` multi-locale (`translations` map) + `resolve` /
  `from_i18n_key` helpers with guestLang → tag → en → fr fallback (wire-compatible
  with `{fr,en}`)

## [2.1.0](https://github.com/PortakiApp/portaki-sdk/compare/v2.0.1...v2.1.0) (2026-07-23)


### ⚠ BREAKING CHANGES

* **ids:** boundary builders no longer accept bare `&str` / `String` where a
  typed id exists. Use [`SurfaceId`], [`OperationName`], [`ModuleId`],
  [`EventType`], [`CapabilityId`], [`NavigateTarget`].
* **action:** `Action::command(module_id, name, args)` takes `&ModuleId` +
  [`OperationName`] (not `impl Into<String>`).
* **action:** `Action::open_overlay(..., surface_render, ...)` takes
  [`SurfaceId`] only.
* **action:** `Action::navigate(to, params)` takes
  [`NavigateTarget`] / [`SurfaceId`] (via `From`) — not free `String`.
  Dynamic shell routes use `NavigateTarget::path(...)`.
* **action:** `Action::emit(event, payload)` takes [`EventType`] only.
* **surface:** `Surface::with_id` takes [`SurfaceId`] only.
* **host:** `events::emit` takes [`EventType`] only.
* **host:** `module::list_by_capability` and `capabilities::has` take
  [`CapabilityId`] only.
* **context:** `Context::has_capability` takes [`CapabilityId`] only;
  `Context::module_id` is [`ModuleId`].
* **ids:** removed `From<&str>` / `From<String>` for [`ModuleId`]. Construct
  with `ModuleId::new` / `ModuleId::from_static` at declaration / test sites.

### Features

* **ids:** newtypes [`SurfaceId`], [`OperationName`], [`ModuleId`],
  [`EventType`] (serde string wire) plus
  `define_surface_ids!` / `define_operation_names!` / `define_event_types!`
* **ids:** shared booklet conventions under [`ids::convention`]
  (`HOME_CARD`, `EXPLORE_DETAIL`, `HOST_MAIN`)
* **action:** [`NavigateTarget`] (`Surface` | `Path`) for typed navigation
* **contracts:** SDK-owned cross-module catalogs —
  `contracts::smart_lock` (capability + `unlock` / `getGuestCredential`),
  `contracts::shell::SURFACE_INPUT`, `contracts::platform::BOOKING_CONFIRMED`
* **macros:** `#[surface]` / `#[command]` / `#[query]` / `#[event_handler]`
  accept `Type::new("…")` wire literals in addition to bare `"…"`

### Documentation

* **docs:** [typed-ids.md](docs/typed-ids.md) — declare once, typed consts at
  every use site
* **docs:** [module-layout.md](docs/module-layout.md) — SDK crate modules
  and Wasm module `guest/` / `host/` / `connectors` / `ids` conventions
* **templates:** empty-module ships `ids.rs` + layout notes aligned with
  guest/host/`ids` conventions

### Refactor

* **organization:** default module template splits guest / host surfaces and
  documents `ids.rs` catalogs (see module-layout)

## [2.0.1](https://github.com/PortakiApp/portaki-sdk/compare/v2.0.0...v2.0.1) (2026-07-23)


### Features

* **action:** `Action::command` takes `impl Serialize` (typed DTOs / [`EmptyArgs`])
* **action:** add [`EmptyArgs`] (`{}`) and [`json_value`] for navigate/emit payloads

## [2.0.0](https://github.com/PortakiApp/portaki-sdk/compare/v1.0.0...v2.0.0) (2026-07-23)


### ⚠ BREAKING CHANGES

* **sdui:** generated primitive props are typed (`String`, `bool`, `f64`/`u32`,
  `Action`, closed enums, nested structs). Scalar / action setters no longer
  accept `serde_json::Value` — drop `json!` on the common authoring path.
* **capability:** `capability::*` constants are now [`CapabilityId`] (serde
  string wire). `Context::with_capabilities` takes `&[CapabilityId]`. Manifest
  `capabilities.required` / `optional[].id` / `provided` deserialize as
  `CapabilityId`.
* **action:** `Action::OpenOverlay.presentation` is [`OverlayPresentation`]
  (not a raw string). Prefer `Action::open_overlay(...)`.
* **action:** `Action::OpenOverlay.args` is [`OverlayArgs`] (not
  `serde_json::Value`). Prefer `OverlayArgs::new().icon(...).title(...)`.
* **email:** guest-stay modules should filter on [`EmailTemplateKey`] instead of
  ad-hoc template strings.

### Features

* **capability:** add closed `CapabilityId` catalog with `as_str` / `FromStr`
* **email:** add `EmailTemplateKey`, `EmailContextArgs`, contribution docs
* **sdui:** typed codegen from `sdui_primitives.json` (`fields` map)
* **sdui:** nested types — `MapViewport`, `MapMarker`, `ChoiceOption`,
  `TemperatureUnit`, `RichTextDoc`, animation / visibility enums
* **action:** `OverlayPresentation`, `OverlayArgs`, `Action::open_overlay`

## [1.0.0](https://github.com/PortakiApp/portaki-sdk/compare/v0.2.1...v1.0.0) (2026-07-21)


### ⚠ BREAKING CHANGES

* **sdk:** host::credentials, images, notifications, repo::update,

### Features

* **capability:** add ai.guest.assistant ([2175387](https://github.com/PortakiApp/portaki-sdk/commit/2175387fc8f9123ff7565c400f28d37315b232f7))
* **cli:** emit operations.bundle v2 schema ([9b66db3](https://github.com/PortakiApp/portaki-sdk/commit/9b66db3bf96f4a7a85da47b6f96fd8db2bcfba5a))
* **connectors:** enrich OpenWeather current and forecast ([7b7f745](https://github.com/PortakiApp/portaki-sdk/commit/7b7f745c6645812bea9a05bef6b4043226ecb7e1))
* **connectors:** expose precip chance and wind speed ([78f7f85](https://github.com/PortakiApp/portaki-sdk/commit/78f7f85630cffdb4cf2f8257d75f14c7eb93031a))
* **sdk:** add provided caps and listByCapability ([c81a1b7](https://github.com/PortakiApp/portaki-sdk/commit/c81a1b75c9de9f84b72c08ddb70414a69ab25153))
* **sdk:** add StayContext for guest reveal ([c83d079](https://github.com/PortakiApp/portaki-sdk/commit/c83d07977c95f19d38ae496c1cb8f602d15f6f3a))
* **sdk:** pass host params as Context.input ([f8c8273](https://github.com/PortakiApp/portaki-sdk/commit/f8c82732933ebfb3ae8fc82af10c6fd5ca11777d))
* **sdk:** remove stub host APIs ([6f271f5](https://github.com/PortakiApp/portaki-sdk/commit/6f271f5fc95f6e47fe3e0751c38046e89f592aae))
* **sdui:** add Card.subtitle and ChoiceList.layout ([6bfed5e](https://github.com/PortakiApp/portaki-sdk/commit/6bfed5e953f981cce0e20dca1831d81dfb2a2674))
* **sdui:** add host form primitives ([da8e9fb](https://github.com/PortakiApp/portaki-sdk/commit/da8e9fbdb8f8b29e87844db4390a8afa5c1d06e2))
* **sdui:** add Stack/Grid/Card layout fields ([d757331](https://github.com/PortakiApp/portaki-sdk/commit/d757331e779cac137ece3cb48859b9aa51eb68e9))
* **sdui:** extend guest primitives for booklet redesign ([a80793f](https://github.com/PortakiApp/portaki-sdk/commit/a80793f1e3d72aeae6e903c1263ae39fe9242043))


### Bug Fixes

* **connectors:** use checked_div for humidity avg ([8ee2291](https://github.com/PortakiApp/portaki-sdk/commit/8ee2291f10f58cf3c617544436749f63109644f6))
* **wasm:** read property lat/lng from configJson ([c93479d](https://github.com/PortakiApp/portaki-sdk/commit/c93479d7ad22f71c047ac5c6d12b5b895228d14d))

## [0.2.1](https://github.com/PortakiApp/portaki-sdk/compare/v0.2.0...v0.2.1) (2026-07-15)


### Bug Fixes

* **publish:** drop sdk↔test-utils publish cycle ([0eb8e16](https://github.com/PortakiApp/portaki-sdk/commit/0eb8e1634600e6ccb905ecd4ce6feffcbed740d2))

## [0.2.0](https://github.com/PortakiApp/portaki-sdk/compare/v0.1.0...v0.2.0) (2026-07-15)


### Features

* **host:** add module.status readiness snapshot ([4ca2e6b](https://github.com/PortakiApp/portaki-sdk/commit/4ca2e6b56a50b2286f710bc780b47f54e4969faa))


### Bug Fixes

* **ci:** rustfmt auth + add quality and release-please ([5b0038a](https://github.com/PortakiApp/portaki-sdk/commit/5b0038af6624514d86b9b61999cf3a7a6987c6f3))
* **deps:** drop invalid Renovate rustMonorepo preset ([65b6357](https://github.com/PortakiApp/portaki-sdk/commit/65b63573bd2474c56a4a6a3a63bf323e9a1d2f25))
* **docs:** indent rustdoc list continuation for clippy ([bb88ee5](https://github.com/PortakiApp/portaki-sdk/commit/bb88ee571e6ed7616df5f4372767a8aff5125a73))

## [0.1.0]

### Features

- Initial open-source SDK workspace (host functions, SDUI, connectors, CLI)
- `host::module::status` readiness snapshot for Wasm modules
