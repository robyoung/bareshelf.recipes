use crate::{
    datatypes::{Ingredient, Recipe},
    error::Result,
    ingredients_schema, recipes_schema,
};

pub struct Indexer {
    recipes_writer: tantivy::IndexWriter,
    recipes_title: tantivy::schema::Field,
    recipes_slug: tantivy::schema::Field,
    recipes_url: tantivy::schema::Field,
    recipes_chef_name: tantivy::schema::Field,
    recipes_image_name: tantivy::schema::Field,
    recipes_ingredient_name: tantivy::schema::Field,
    recipes_ingredient_slug: tantivy::schema::Field,

    ingredients_writer: tantivy::IndexWriter,
    ingredients_name: tantivy::schema::Field,
    ingredients_slug: tantivy::schema::Field,
}

impl Indexer {
    pub(crate) fn new(recipes: &tantivy::Index, ingredients: &tantivy::Index) -> Result<Indexer> {
        let recipes_schema = recipes_schema();
        let ingredients_schema = ingredients_schema();
        Ok(Indexer {
            recipes_writer: recipes.writer(30_000_000)?,
            recipes_title: recipes_schema.get_field("title").unwrap(),
            recipes_slug: recipes_schema.get_field("slug").unwrap(),
            recipes_url: recipes_schema.get_field("url").unwrap(),
            recipes_chef_name: recipes_schema.get_field("chef_name").unwrap(),
            recipes_image_name: recipes_schema.get_field("image_name").unwrap(),
            recipes_ingredient_name: recipes_schema.get_field("ingredient_name").unwrap(),
            recipes_ingredient_slug: recipes_schema.get_field("ingredient_slug").unwrap(),

            ingredients_writer: ingredients.writer(30_000_000)?,
            ingredients_name: ingredients_schema.get_field("name").unwrap(),
            ingredients_slug: ingredients_schema.get_field("slug").unwrap(),
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
        document.add_text(self.recipes_title, &recipe.title);
        document.add_text(self.recipes_slug, &recipe.slug);
        document.add_text(self.recipes_url, &recipe.url);
        if let Some(chef_name) = &recipe.chef_name {
            document.add_text(self.recipes_chef_name, &chef_name);
        }
        if let Some(image_name) = &recipe.image_name {
            document.add_text(self.recipes_image_name, &image_name);
        }

        recipe.ingredients.iter().for_each(|ingredient| {
            document.add_text(self.recipes_ingredient_name, &ingredient.name);
            document.add_facet(
                self.recipes_ingredient_slug,
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
        document.add_text(self.ingredients_name, &ingredient.name);
        document.add_text(self.ingredients_slug, &ingredient.slug);
        document
    }
}
