build:
	@cargo build

build-release:
	@cargo build --release

install:
	@cp target/release/ansible-inline-vaulter ~/bin/ansible-inline-vaulter

clean:
	-@rm -rf target/