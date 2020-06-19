use std::path::Path;
use tantivy::schema::{STORED, STRING, TEXT};

mod datatypes;
mod error;
mod indexer;
mod next_ingredient;
mod searcher;

pub use crate::{
    datatypes::{Ingredient, IngredientSlug, Recipe},
    error::{Error, Result},
    indexer::Indexer,
    searcher::{IngredientQuery, RecipeQuery, RecipeSearchResult, Searcher},
};

pub fn indexer(path: &Path) -> Result<Indexer> {
    Indexer::new(
        &open_or_create_index(path.join("recipes").as_path(), recipes_schema())?,
        &open_or_create_index(path.join("ingredients").as_path(), ingredients_schema())?,
    )
}

pub fn searcher(path: &Path) -> Result<Searcher> {
    Searcher::new(
        &open_index(path.join("recipes").as_path())?,
        &open_index(path.join("ingredients").as_path())?,
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
    schema_builder.add_text_field("url", STORED);
    schema_builder.add_text_field("image_name", STORED);
    schema_builder.add_text_field("chef_name", STORED);
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
pub(crate) mod tests {
    use super::*;

    pub(crate) fn setup_recipes_index() -> (tantivy::Index, tantivy::Index) {
        let (recipes_index, ingredients_index) = create_indexes();

        index_recipes(&recipes_index, &ingredients_index);

        (recipes_index, ingredients_index)
    }

    pub(crate) fn setup_ingredients_index() -> (tantivy::Index, tantivy::Index) {
        let (recipes_index, ingredients_index) = create_indexes();

        index_ingredients(&recipes_index, &ingredients_index);

        (recipes_index, ingredients_index)
    }

    pub(crate) fn setup_indexes() -> (tantivy::Index, tantivy::Index) {
        let (recipes_index, ingredients_index) = create_indexes();

        index_recipes(&recipes_index, &ingredients_index);
        index_ingredients(&recipes_index, &ingredients_index);

        (recipes_index, ingredients_index)
    }

    pub(crate) fn create_indexes() -> (tantivy::Index, tantivy::Index) {
        (
            tantivy::Index::create_in_ram(recipes_schema()),
            tantivy::Index::create_in_ram(ingredients_schema()),
        )
    }

    fn index_recipes(recipes_index: &tantivy::Index, ingredients_index: &tantivy::Index) {
        let mut indexer = Indexer::new(&recipes_index, &ingredients_index).unwrap();

        indexer.add_recipe(Recipe::new(
            "Fried egg",
            "fried-egg",
            "http://example.org/one",
            vec![Ingredient::new("Egg", "egg"), Ingredient::new("Oil", "oil")],
        ));
        indexer.add_recipe(Recipe::new(
            "Scrambled egg",
            "scrambled-egg",
            "http://example.org/two",
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
            "http://example.org/three",
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
    }

    fn index_ingredients(recipes_index: &tantivy::Index, ingredients_index: &tantivy::Index) {
        let mut indexer = Indexer::new(&recipes_index, &ingredients_index).unwrap();
        indexer.add_ingredient(Ingredient::new("Peanut butter", "peanut-butter"));
        indexer.add_ingredient(Ingredient::new("Sugar", "sugar"));
        indexer.add_ingredient(Ingredient::new("Egg", "egg"));
        indexer.add_ingredient(Ingredient::new("Butter", "butter"));
        indexer.add_ingredient(Ingredient::new("Butter beans", "butter-beans"));
        indexer.add_ingredient(Ingredient::new("Brown sugar", "brown-sugar"));
        indexer.add_ingredient(Ingredient::new("Garlic", "garlic"));
        indexer.add_ingredient(Ingredient::new("Milk", "milk"));
        indexer.add_ingredient(Ingredient::new("Salt", "salt"));
        indexer.add_ingredient(Ingredient::new("Oil", "oil"));
        indexer.add_ingredient(Ingredient::new("Tortilla wrap", "tortilla-wrap"));
        indexer.add_ingredient(Ingredient::new("Mushroom", "mushroom"));
        indexer.commit().unwrap();
    }
}
