`gonzales` has no unsafe code and is very fast.

The benchmarks include 130 routes per iteration.

#Regular matching
```
test gonzales ... bench:                363 ns/iter (+/- 15)
test matchit ... bench:                 403 ns/iter (+/- 12)
test bench_regex_set ... bench:         39604 ns/iter (+/- 2274)
test bench_actix ... bench:             55248 ns/iter (+/- 2905)
test bench_route_recognizer ... bench:  13219 ns/iter (+/- 564)
```

#Case insensitive matching
```
test gonzales ... bench:         423 ns/iter (+/- 11)
test matchit ... bench:        3921 ns/iter (+/- 104)
test actix ... bench:       54038 ns/iter (+/- 1484)
test regex ... bench:       33513 ns/iter (+/- 3388)
test route-recognizer ... bench:       13622 ns/iter (+/- 1177)
```

```rust
        let route = vec!["/hello/{user_id}", "/helloworld"];
        let router = RouterBuilder::new()
            .ascii_case_insensitive(true)
            .build(route);
        let m = router.route("/HelloWorld")?;

        // the index of the matched route
        let index = m.get_index();
        assert_eq!(1, index);
        
        // route arguments
        let args = m.get_args();
        assert!(args.is_empty());

        // segments matched with `*`
        let segments = m.get_segments();
        assert!(segments.is_empty());
        
```

The router also supports multi-segment matching with `*`, only at the end of a route.
`/hello/world/*` will match every incoming request path that starts with `/hello/world/`.

arriba arriba andale andale!!!
