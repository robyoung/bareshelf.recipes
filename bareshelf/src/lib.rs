use std::path::Path;
use tantivy::schema::{STORED, STRING, TEXT};

mod error;
mod indexer;
mod searcher;

pub use crate::{
    error::Result,
    indexer::{Indexer, Ingredient, Recipe},
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
        std::fs::create_dir_all(path)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestIndex(String);

    impl Drop for TestIndex {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn tweak_score_with_facets() {
        let data = TestIndex("/tmp/bareshelf/test-index".to_string());
        let mut indexer = indexer(Path::new(&data.0)).unwrap();
        indexer.add_recipe(Recipe::new(
            "Fried egg",
            "fried-egg",
            vec![Ingredient::new("Egg", "egg"), Ingredient::new("Oil", "oil")],
        ));
        indexer.add_recipe(Recipe::new(
            "Scrambled egg",
            "scrambled-egg",
            vec![
                Ingredient::new("Egg", "egg"),
                Ingredient::new("Butter", "butter"),
                Ingredient::new("Milk", "milk"),
                Ingredient::new("Salt", "salt"),
            ],
        ));
        indexer.add_recipe(Recipe::new(
            "Egg rolls",
            "egg-rolls",
            vec![
                Ingredient::new("Egg", "egg"),
                Ingredient::new("Garlic", "garlic"),
                Ingredient::new("Salt", "salt"),
                Ingredient::new("Oil", "oil"),
                Ingredient::new("Tortilla wrap", "tortilla-wrap"),
                Ingredient::new("Mushroom", "mushroom"),
            ],
        ));
        indexer.commit().unwrap();

        let searcher = searcher(Path::new(&data.0)).unwrap();
        let query_ingredients = vec!["egg", "oil", "garlic", "mushroom"];
        let results = searcher.recipes_by_ingredients(&query_ingredients, 2).unwrap();

        assert_eq!(results.iter().map(|r| r.recipe_title.to_owned()).collect::<Vec<String>>(), vec!["Fried egg", "Egg rolls"]);

    }
}
