#![recursion_limit = "128"]

use criterion::*;
use gxi_linecache::LineCache;
use serde_json::json;

fn bench_ins(c: &mut Criterion) {
    let json = json!({
        "ops": [
            {"op":"invalidate", "n": 20},
            {"op":"ins", "n": 30, "lines": [
                {"text": "20\n", "ln": 1},
                {"text": "21\n"},
            ]},
            {"op":"invalidate", "n": 10},
            {"op":"ins", "n": 30, "lines": [
                {"text": "32\n"},
                {"text": "33\n"},
            ]},
            {"op":"invalidate", "n": 10},
            {"op":"ins", "n": 30, "lines": [
                {"text": "44\n"},
                {"text": "45\n"},
                {"text": "46\n"},
                {"text": "47\n"},
            ]},
            {"op":"ins", "n": 30, "lines": [
                {"text": "48\n"},
                {"text": "49\n"},
                {"text": "50\n"},
                {"text": "51\n"},
                {"text": "52\n"},
            ]},
            {"op":"invalidate", "n": 10},
        ]
    });

    c.bench_function("linecache_ins", move |b| {
        b.iter(|| {
            let mut lc = LineCache::new();
            lc.apply_update(&json)
        })
    });
}

fn bench_inv_copy(c: &mut Criterion) {
    let json = json!({
        "ops": [
            {"n":10,"op":"invalidate"},
            {"n":10,"op":"invalidate"},
            {"n":20,"op":"skip"},
            {"n":2,"op":"copy"},
            {"n":10,"op":"invalidate"},
            {"n":10,"op":"skip"},
            {"n":3,"op":"copy"},
            {"n":10,"op":"invalidate"},
            {"n":10,"op":"skip"},
            {"n":4,"op":"copy"},
            {"n":5,"op":"copy"},
            {"n":2,"op":"ins",
                "lines":[
                    {"styles":[0,70,2],"text":"53\n"},
                    {"styles":[0,72,2],"text":"54\n"},
            ]},
            {"n":8,"op":"invalidate"},
        ]
    });

    c.bench_function("linecache_inv_copy", move |b| {
        b.iter(|| {
            let mut lc = LineCache::new();
            lc.apply_update(&json)
        })
    });
}

criterion_group!(benches, bench_ins, bench_inv_copy);
criterion_main!(benches);
