
build-search:
	cargo build -p search --release
	cp ./target/release/libsearch.so ./admin/bareshelf_admin/search.so
