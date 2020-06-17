test:
	cd bareshelf && cargo test
	cd bareshelf_web && cargo test
	cd bareshelf_indexer && cargo test --no-default-features

build-indexer:
	cargo build -p bareshelf_indexer --release
	cp ./target/release/libbareshelf_indexer.so ./admin/bareshelf_admin/bareshelf_indexer.so

crawl-ingredients:
	dc exec admin scrapy crawl bbc-ingredients

crawl-recipes:
	dc exec admin scrapy crawl bbc-recipes

crawl-all: crawl-ingredients crawl-recipes

run-indexer: build-indexer
	sudo chmod -R 777 search-index
	dc exec admin flask index
	sudo chown -R robyoung:robyoung search-index

build-web:
	cd bareshelf_web && cargo build --features embedded-templates ---release --target-dir ../target

deploy-web:
	scp target/release/bareshelf_web $(WEB_DEPLOY_TARGET):
	ssh $(WEB_DEPLOY_TARGET) "systemctl stop bareshelf_web.service && mv bareshelf_web /usr/local/bin/ && systemctl start bareshelf_web.service"

.PHONY: build-indexer run-indexer build-web
