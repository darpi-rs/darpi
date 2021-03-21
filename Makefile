.PHONY: publish
publish:
	cd darpi-route/ && cargo publish
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