use crate::{
    datatypes::{Ingredient, Recipe},
    error::Result,
    ingredients_schema, recipes_schema,
};

pub struct Indexer {
    recipes_writer: tantivy::IndexWriter,
    // TODO: don't do this as the clones aren't expensive
    recipes_fields: [tantivy::schema::Field; 6],

    ingredients_writer: tantivy::IndexWriter,
    ingredients_fields: [tantivy::schema::Field; 2],
}

impl Indexer {
    pub(crate) fn new(recipes: &tantivy::Index, ingredients: &tantivy::Index) -> Result<Indexer> {
        let recipes_schema = recipes_schema();
        let ingredients_schema = ingredients_schema();
        Ok(Indexer {
            recipes_writer: recipes.writer(30_000_000)?,
            recipes_fields: [
                recipes_schema.get_field("title").unwrap(),
                recipes_schema.get_field("slug").unwrap(),
                recipes_schema.get_field("url").unwrap(),
                recipes_schema.get_field("chef_name").unwrap(),
                recipes_schema.get_field("ingredient_name").unwrap(),
                recipes_schema.get_field("ingredient_slug").unwrap(),
            ],
            ingredients_writer: ingredients.writer(30_000_000)?,
            ingredients_fields: [
                ingredients_schema.get_field("name").unwrap(),
                ingredients_schema.get_field("slug").unwrap(),
            ],
        })
    }

    pub fn commit(&mut self) -> Result<()> {
        self.recipes_writer.commit()?;
        self.ingredients_writer.commit()?;
        Ok(())
    }

    pub fn add_recipe(&mut self, recipe: Recipe) {
        self.recipes_writer
            .add_document(self.create_recipe_doc(&recipe));
    }

    fn create_recipe_doc(&self, recipe: &Recipe) -> tantivy::schema::Document {
        let mut document = tantivy::schema::Document::default();
        document.add_text(self.recipes_fields[0], &recipe.title);
        document.add_text(self.recipes_fields[1], &recipe.slug);
        document.add_text(self.recipes_fields[2], &recipe.url);
        if let Some(chef_name) = &recipe.chef_name {
            document.add_text(self.recipes_fields[3], &chef_name);
        }

        recipe.ingredients.iter().for_each(|ingredient| {
            document.add_text(self.recipes_fields[4], &ingredient.name);
            document.add_facet(
                self.recipes_fields[5],
                &format!("/ingredient/{}", ingredient.slug),
            );
        });
        document
    }

    pub fn add_ingredient(&mut self, ingredient: Ingredient) {
        self.ingredients_writer
            .add_document(self.create_ingredient_doc(&ingredient));
    }

    fn create_ingredient_doc(&self, ingredient: &Ingredient) -> tantivy::schema::Document {
        let mut document = tantivy::schema::Document::default();
        document.add_text(self.ingredients_fields[0], &ingredient.name);
        document.add_text(self.ingredients_fields[1], &ingredient.slug);
        document
    }
}
