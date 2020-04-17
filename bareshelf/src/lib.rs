use std::path::Path;
use tantivy::schema::{STRING, TEXT, STORED};

mod error;
mod indexer;
mod searcher;

pub use crate::{
    error::Result,
    indexer::{Indexer, Recipe, Ingredient},
    searcher::Searcher,
};

pub fn indexer(path: &Path) -> Result<Indexer> {
    Indexer::new(
        open_or_create_index(path.join("recipes").as_path(), recipes_schema())?,
        open_or_create_index(path.join("ingredients").as_path(), ingredients_schema())?,
    )
}

pub fn searcher(path: &Path) -> Result<Searcher> {
    Searcher::new(
        open_index(path.join("recipes").as_path())?,
        open_index(path.join("ingredients").as_path())?,
    )
}

fn open_index(path: &Path) -> Result<tantivy::Index> {
    let directory = tantivy::directory::MmapDirectory::open(path)?;
    Ok(tantivy::Index::open(directory)?)
}

fn open_or_create_index(path: &Path, schema: tantivy::schema::Schema) -> Result<tantivy::Index> {
    if !path.exists() {
        std::fs::create_dir(path)?;
    }
    let directory = tantivy::directory::MmapDirectory::open(path)?;
    Ok(tantivy::Index::open_or_create(directory, schema)?)
}

fn recipes_schema() -> tantivy::schema::Schema {
    let mut schema_builder = tantivy::schema::Schema::builder();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("slug", STRING | STORED);
    schema_builder.add_facet_field("ingredient_slug");
    schema_builder.add_text_field("ingredient_name", TEXT | STORED);
    schema_builder.build()
}

fn ingredients_schema() -> tantivy::schema::Schema {
    let mut schema_builder = tantivy::schema::Schema::builder();
    schema_builder.add_text_field("name", TEXT | STORED);
    schema_builder.add_text_field("slug", STRING | STORED);
    schema_builder.build()
}
