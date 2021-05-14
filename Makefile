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