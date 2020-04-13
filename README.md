# Bare shelf recipes

A recipe search for making food with what you've got.

Find recipes to make with what you've got in your cupboard.

## Components

### Admin

This is the admin backend for the site. It is made up of a [Flask-Admin](https://flask-admin.readthedocs.io/en/latest/)
site using [Flask-SQLAchemy](https://flask-sqlalchemy.palletsprojects.com/en/2.x/) for the models and a [Scrapy](https://scrapy.org/)
scraper. It is responsible for scraping recipes and ingredients into the database and then indexing them into the search.

### Search

Search uses [Tantivy](https://github.com/tantivy-search/tantivy). An indexing interface is exposed to the Admin with
[PyO3](https://github.com/PyO3/pyo3). The search index is the artefact that is deployed along with the Web app.

It is split across two rust crates; `indexer` is a PyO3 libary used by Admin to index all the recipes and ingredients,
`search` is a crate for querying the index used by Web.

### Web

This is the web front end that performs searches against the search index.
