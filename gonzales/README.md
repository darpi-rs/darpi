`gonzales` is the fastest http router in the business and has no unsafe code.

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