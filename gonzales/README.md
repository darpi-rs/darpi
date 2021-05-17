`gonzales` is the fastest http router in the business and has no unsafe code.

The benchmarks include 130 routes per iteration.
```
test gonzales ... bench:                363 ns/iter (+/- 15)
test matchit ... bench:                 403 ns/iter (+/- 12)
test bench_regex_set ... bench:         39604 ns/iter (+/- 2274)
test bench_actix ... bench:             55248 ns/iter (+/- 2905)
test bench_route_recognizer ... bench:  13219 ns/iter (+/- 564)
```

It costs `1ns` per character of input for matching.
That leads to most routes being matched or not within `10ns` to `20ns`.
It supports case insensitive matching without additional runtime costs.
It also supports path arguments extraction.

Under the hood, it uses a [DFA](https://en.wikipedia.org/wiki/Deterministic_finite_automaton) with a few extra perks.
Many thanks to [BurntSushi](https://github.com/BurntSushi) for his work in this field and his great articles such as [this](https://blog.burntsushi.net/transducers/) .

```rust
        let route = vec!["/hello/{user_id}", "/helloworld"];
        let router = RouterBuilder::new()
            .ascii_case_insensitive(true)
            .build(route);
        let m = router.route("/HelloWorld");
        assert_eq!(
            Some(Match {
                index: 1,
                args: vec![],
                multi_segments: vec![]
            }),
            m
        );
```

The router also supports multi-segment matching with `*`, only at the end of a route.
`/hello/world/*` will match every incoming request path that starts with `/hello/world/`.

arriba arriba andale andale!!!