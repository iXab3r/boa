# Fork patches

Upstream: https://github.com/boa-dev/boa.
Base: `337a3668a0dc86dd401ea20906e782249a64a228` (tag `v0.22`, crates 0.22.0).
Branches: `main` = base + the changes below; `upstream` = mirror of boa-dev/boa `main`.
Full diff: [`v0.22...main`](https://github.com/iXab3r/boa/compare/337a3668a0dc86dd401ea20906e782249a64a228...main).

Only the four changes below remain. Commit `3ed647f5` removed an earlier copy of these notes and
changes no code. Keep the summary list in README.md in sync with this file.

| Change | Commits | Upstream status |
|---|---|---|
| [Atomic `Symbol.for` registration](#atomic-symbolfor-registration) | `1de91fb9` (engine source and test) | To be proposed upstream; not submitted yet |
| [Host-call operand release](#host-call-operand-release) | `54426b70` | To be proposed upstream; not submitted yet |
| [Module namespace tracing](#module-namespace-tracing) | `9a82346f` | To be proposed upstream; not submitted yet |
| [Crate identity alignment](#crate-identity-alignment) | `1de91fb9` (engine manifest), `5e9ff9bb` | Fork only: needed for embedding, not an upstream bug |

GitHub Actions is disabled in this fork; evidence comes from local runs.

## Atomic `Symbol.for` registration

- Why: the global symbol registry is shared by every context in the process. `Symbol.for` looked a
  key up and inserted it in two separate steps, so contexts on different threads registering the same
  new key at the same time could receive different symbols, and `Symbol.for(key) === Symbol.for(key)`
  stopped holding across contexts.
- What: `core/engine/src/builtins/symbol/mod.rs` interns the key under one DashMap entry guard. The
  reverse `Symbol.keyFor` entry is installed before the canonical symbol is published. Existing symbols
  and unrelated keys keep their identities.
- Verified: `core/engine/tests/symbol_registry.rs` registers 64 fresh keys simultaneously from 16
  contexts on separate threads and checks one identity per key, reverse lookup and distinct identities
  for distinct keys.
  Local run 2026-11-01 (Windows, Rust 1.97.0): passes on `main`; fails in 5 of 5 runs with only
  `builtins/symbol/mod.rs` restored to `v0.22`.

## Host-call operand release

- Why: `JsObject::call` pushes the callee, `this` and the arguments onto the VM stack before dispatch.
  When dispatch failed before a frame existed (a revoked proxy, a proxy whose `apply` getter throws),
  those operands stayed on the stack, so the context kept discarded receivers and arguments alive.
- What: `core/engine/src/object/operations.rs` records the stack length on entry to `JsObject::call`
  and truncates back to it on every exit through a context guard, preserving the operands of callers
  in nested host calls. `core/engine/src/vm/mod.rs` adds crate-private `Stack::len`/`Stack::truncate`.
- Verified: consumer regression tests in EyeAuras.Scripting call a revoked proxy and a proxy whose
  `apply` getter throws, through a callback and a property getter, and check that the receiver and the
  arguments are collected afterwards while the context keeps evaluating. There is no engine-level test in
  this fork yet; an upstream proposal needs one.
  Local run 2026-11-01 (Windows) of those two consumer tests against `5e9ff9bb`: both pass. They were
  not re-run against the unfixed engine on that date.

## Module namespace tracing

- Why: `ModuleNamespace` caches resolved export bindings, and each `ResolvedBinding` holds a `Module`
  handle. The cache was marked `#[unsafe_ignore_trace]` on the assumption that those modules are reachable
  through the module graph anyway. Boa's GC counts non-root references during tracing, so untraced handles
  are counted as roots: a namespace kept its module graph alive after everything else had released it.
- What: `core/engine/src/module/namespace.rs` traces `resolved_bindings`; `core/engine/src/module/mod.rs`
  derives `Trace`/`Finalize` for `ResolvedBinding` and ignores only its `BindingName`, which holds a
  `JsString` or a unit variant and no GC handle.
- Verified: the unit test `module::namespace::gc_tests::cached_export_bindings_do_not_root_unreachable_module_graphs`
  keeps an exported object alive while the context lives and checks that it is finalized after the
  context is dropped.
  Local run 2026-11-01 (Windows, Rust 1.97.0): passes on `main`; fails with only the namespace/module
  source restored to `v0.22` ("cached export handles must not become permanent roots").

## Crate identity alignment

- Why: consumers take `boa_engine` and `boa_runtime` from this fork by Git revision and every other Boa
  crate from crates.io. Upstream manifests resolve sibling crates through workspace paths, which would
  bring a second `boa_gc`, `boa_parser`, `boa_string` and so on from the Git checkout. Two GC crates mean
  two incompatible GC heaps and duplicate types.
- What: `core/engine/Cargo.toml` declares its workspace explicitly and takes `boa_interner`, `boa_gc`,
  `boa_macros`, `boa_ast`, `boa_parser`, `boa_string`, `boa_icu_provider` (=0.22.0), `tag_ptr` and
  `small_btree` (=0.1.0) from crates.io at the base release versions. `core/runtime/Cargo.toml` and
  `core/wintertc/Cargo.toml` take `boa_gc` =0.22.0 from crates.io and keep the fork's engine.
- Verified: EyeAuras.Scripting builds `--locked` against `5e9ff9bb` and `cargo tree` resolves one
  `boa_engine` and one `boa_runtime` (both from this fork) and one `boa_gc` (crates.io); `cargo tree -d`
  shows no duplicate Boa crate (local run 2026-11-01).
- Upstream: not proposed. Upstream builds all crates from one workspace; this is only needed when a
  consumer mixes Git and registry Boa crates.

## Updating from upstream

This procedure is the same in the tsrs and Boa forks; only the checks differ.

`upstream` mirrors the upstream default branch and never receives our commits. `main` is the
upstream base plus the changes listed in this file. Consumers pin `main` commits, so the history
of `main` only grows: never rebase, squash or force-push it.

1. Refresh the mirror: `git fetch upstream --tags`, then fast-forward the fork's `upstream` branch
   (`git push origin upstream/main:refs/heads/upstream`). If that is not a fast-forward, stop and ask:
   upstream rewrote its history.
2. Pick the target: the newest upstream release tag, or an upstream `main` commit when a needed fix
   is not tagged yet.
3. Branch from `main` (`sync/upstream-<target>`) and `git merge <target>`. Resolve conflicts inside
   our changes; elsewhere keep the upstream code.
4. Review every change in this file. If upstream now contains it (our accepted upstream PR or an
   equivalent fix), drop our version in the merge. Otherwise confirm the reason still holds and the
   change still fits the new code.
5. Check new upstream behavior that matters to an embedding host (see the checks below).
6. Run the checks locally. GitHub Actions is disabled in this fork; put the commands and results in
   the PR description.
7. Update the base, the change list in README.md and the status table in this file.
8. Open a PR into `main` and merge it with a merge commit.
9. Re-pin consumers in a separate change in each consumer repository, with its own tests and
   `--locked` builds. Old pins stay reachable from `main`.

Cadence: when an upstream release contains fixes we need, or monthly, whichever comes first.
Syncing upstream is a routine operation. Adding or changing our own patches needs the owner's
explicit approval in the PR.

Checks for Boa (MSRV from `rust-version` in Cargo.toml):

- Target: a release tag only. The support crates come from crates.io at the base release version, so an
  untagged upstream commit has no matching registry crates.

- `cargo test -p boa_engine`, including `--test symbol_registry` and `--lib gc_tests`
- `cargo clippy --all-targets -- -D warnings`
- Crate identity: the engine, runtime and wintertc manifests still take the support crates (`boa_gc`,
  `boa_parser`, `boa_ast`, `boa_string`, `boa_interner`, ...) from crates.io at the release version of
  the new base. Update those exact versions together with the base.
- Consumer re-pin: `boa_engine` and `boa_runtime` come from the same fork commit, and `cargo tree -d`
  shows one `boa_engine` and one `boa_gc`.
