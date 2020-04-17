
build-indexer:
	cargo build -p bareshelf_indexer --release
	cp ./target/release/libindexer.so ./admin/bareshelf_admin/bareshelf_indexer.so

run-indexer: build-indexer
	sudo chmod -R 777 search-index
	dc exec admin flask index
	sudo chown -R robyoung:robyoung search-index

.PHONY: build-indexer run-indexer
