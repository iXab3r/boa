# Retained patch over Boa v0.22

Base: `337a3668a0dc86dd401ea20906e782249a64a228` (`v0.22`).

`Symbol.for` now interns a key under one DashMap entry guard. The previous separate
lookup and insert allowed simultaneous contexts to receive distinct symbols for
one key. The reverse `Symbol.keyFor` entry is installed before publishing the
canonical symbol. Existing symbols and unrelated keys preserve their identities.

`core/engine/tests/symbol_registry.rs` exercises simultaneous first registration
from 16 contexts across 64 fresh keys, reverse lookup, and distinct-key identity.

The engine manifest resolves its sibling and utility crates from crates.io at
the original release versions. This allows consumers to patch only `boa_engine`
without duplicating GC, parser, string, or AST crate identities. No other engine
source is changed from v0.22.
