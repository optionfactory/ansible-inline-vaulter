build:
	@cargo build

build-release:
	@cargo build --release

install:
	@cp target/release/ansible-inline-vaulter ~/bin/ansible-inline-vaulter

clean:
	-@rm -rf target/

release-patch: INC=patch
release-patch: _release

release-minor: INC=minor
release-minor: _release

_release: build-release
	cargo release ${INC}
