.PHONY: publish
publish:
	cd gonzales/ && cargo publish
	cd ..
	cd darpi-code-gen/ && cargo publish
	cd ..
	cd darpi-web/ && cargo publish
	cd ..
	cargo publish
	cd darpi-headers/ && cargo publish
	cd ..
	cd darpi-middleware/ && cargo publish
	cd ..
	cd darpi-graphql/ && cargo publish

.PHONY: benches
benches:
	cargo bench --benches -- --output-format  bencher

.PHONY: route
route:
	cargo bench -- --output-format  bencher

.PHONY: pprof
pprof:
	cargo bench -- --profile-time 60

.PHONY: cloc
cloc:
	cloc --exclude-dir=target .

.PHONY: test
test:
	cargo test --all