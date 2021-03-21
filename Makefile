.PHONY: publish
publish:
	cd darpi-route/ && cargo publish
	cd ..
	cd darpi-code-gen/ && cargo publish
	cd ..
	cd darpi-web/ && cargo publish
	cd ..