//! Structs used for rendering templates
//!
use std::collections::HashSet;

use serde::Serialize;

#[derive(Serialize)]
pub struct RecipeSearchResult {
    score: f32,
    title: String,
    url: String,
    source: String,
    chef_name: Option<String>,
    image_name: Option<String>,
    ingredients: Vec<RecipeSearchResultIngredient>,
    num_missing: usize,
}

impl From<bareshelf::RecipeSearchResult> for RecipeSearchResult {
    fn from(recipe: bareshelf::RecipeSearchResult) -> Self {
        let missing: HashSet<_> = recipe.missing_ingredients.iter().collect();
        Self {
            score: recipe.score,
            title: recipe.recipe.title,
            url: recipe.recipe.url.clone(),
            source: url::Url::parse(&recipe.recipe.url)
                .unwrap()
                .host_str()
                .unwrap()
                .to_owned(),
            chef_name: recipe.recipe.chef_name,
            image_name: recipe.recipe.image_name,
            ingredients: recipe
                .recipe
                .ingredients
                .iter()
                .map(|ingredient| RecipeSearchResultIngredient {
                    name: ingredient.name.to_owned(),
                    slug: ingredient.slug.to_owned(),
                    is_missing: missing.contains(&ingredient.slug),
                })
                .collect(),
            num_missing: missing.len(),
        }
    }
}

#[derive(Serialize)]
pub struct RecipeSearchResultIngredient {
    name: String,
    slug: String,
    is_missing: bool,
}
