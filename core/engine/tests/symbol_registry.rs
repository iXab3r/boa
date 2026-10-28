use std::collections::HashSet;
use std::sync::{Arc, Barrier};

use boa_engine::{Context, JsString, JsValue, Source};

#[test]
fn concurrent_contexts_share_one_registered_symbol_per_key() {
    const WORKERS: usize = 16;
    const ROUNDS: usize = 64;
    let barrier = Arc::new(Barrier::new(WORKERS));
    let workers: Vec<_> = (0..WORKERS)
        .map(|_| {
            let barrier = barrier.clone();
            std::thread::Builder::new()
                .stack_size(4 << 20)
                .spawn(move || {
                    let mut context = Context::default();
                    let symbol_for = context
                        .eval(Source::from_bytes("Symbol.for"))
                        .unwrap()
                        .as_callable()
                        .unwrap();
                    let symbol_key_for = context
                        .eval(Source::from_bytes("Symbol.keyFor"))
                        .unwrap()
                        .as_callable()
                        .unwrap();
                    (0..ROUNDS)
                        .map(|round| {
                            let key = JsString::from(format!(
                                "boa.symbol-registry.{}.{}",
                                std::process::id(),
                                round
                            ));
                            barrier.wait();
                            let symbol = symbol_for
                                .call(&JsValue::undefined(), &[key.clone().into()], &mut context)
                                .unwrap();
                            let recovered = symbol_key_for
                                .call(&JsValue::undefined(), &[symbol.clone()], &mut context)
                                .unwrap();
                            (
                                symbol.as_symbol().unwrap().hash(),
                                recovered.as_string().is_some_and(|value| value == key),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap()
        })
        .collect();
    let results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    for round in 0..ROUNDS {
        let identities: Vec<_> = results.iter().map(|result| result[round].0).collect();
        assert!(
            identities.iter().all(|identity| *identity == identities[0]),
            "Symbol.for returned different symbols for the same key in round {round}: {identities:?}"
        );
        assert!(
            results.iter().all(|result| result[round].1),
            "Symbol.keyFor must recover each original key in round {round}"
        );
    }
    let distinct: HashSet<_> = results[0].iter().map(|result| result.0).collect();
    assert_eq!(
        distinct.len(),
        ROUNDS,
        "different keys must retain distinct symbols"
    );
}
