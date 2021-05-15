use criterion::{criterion_group, criterion_main, Criterion};
use gonzales::RouterBuilder;

const MATCHES: &[(&str, &str); 4] = &[
    ("/repos/foo/bar/stargazers", "GET"),
    ("/user/repos", "POST"),
    ("/org/foo/public_members/bar", "GET"),
    ("/hello/petar/world", "GET"),
];

fn dfa_match(c: &mut Criterion) {
    let mut routes = vec![
        "/hello/world".to_string(),
        "/hello/*/world".to_string(),
        "/hello/petar/world".to_string(),
        "/hello/*".to_string(),
    ];
    for i in 0..50 {
        routes.push(format!("/hello/world/{}", i));
    }
    let router = RouterBuilder::new().build(routes.as_slice());

    c.bench_function("dfa_match", |b| {
        b.iter(|| {
            for (m, _) in MATCHES {
                let _ = router.route(m);
            }
        });
    });
}

criterion_group!(benches, dfa_match);
criterion_main!(benches);
