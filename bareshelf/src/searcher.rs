use std::collections::HashSet;

use serde::Serialize;
use tantivy::{
    collector::{Count, FacetCollector, TopDocs},
    fastfield::FacetReader,
    query::{AllQuery, BooleanQuery, FuzzyTermQuery},
    schema::{Facet, Field, Term, Value},
    DocId, Document, IndexReader, Score, SegmentReader,
};

use crate::error::Result;

#[derive(Clone)]
pub struct Searcher {
    recipes_reader: IndexReader,
    recipes_title: Field,
    recipes_url: Field,
    recipes_chef_name: Field,
    recipes_ingredient_slug: Field,

    ingredients_reader: IndexReader,
    ingredients_name: Field,
    ingredients_slug: Field,
}

impl Searcher {
    pub(crate) fn new(recipes: tantivy::Index, ingredients: tantivy::Index) -> Result<Searcher> {
        let recipes_schema = recipes.schema();
        let ingredients_schema = ingredients.schema();

        Ok(Searcher {
            recipes_reader: recipes.reader()?,
            recipes_title: recipes_schema.get_field("title").unwrap(),
            recipes_ingredient_slug: recipes_schema.get_field("ingredient_slug").unwrap(),
            recipes_url: recipes_schema.get_field("url").unwrap(),
            recipes_chef_name: recipes_schema.get_field("chef_name").unwrap(),

            ingredients_reader: ingredients.reader()?,
            ingredients_name: ingredients_schema.get_field("name").unwrap(),
            ingredients_slug: ingredients_schema.get_field("slug").unwrap(),
        })
    }

    pub fn recipe_ingredients(&self) -> Result<Vec<(String, u64)>> {
        let searcher = self.recipes_reader.searcher();
        let mut facet_collector = FacetCollector::for_field(self.recipes_ingredient_slug);
        facet_collector.add_facet("/ingredient");
        let facet_counts = searcher.search(&AllQuery, &facet_collector)?;

        Ok(facet_counts
            .get("/ingredient")
            .map(|(facet, count)| (facet.to_path()[1].to_owned(), count))
            .collect())
    }

    pub fn recipes_by_ingredients(
        &self,
        ingredients: &[String],
        limit: usize,
    ) -> Result<Vec<RecipeSearchResult>> {
        let ingredient_slug_field = self.recipes_ingredient_slug;
        let recipe_title_field = self.recipes_title;
        let recipe_url_field = self.recipes_url;
        let recipe_chef_name_field = self.recipes_chef_name;

        let ingredients: Vec<IngredientSlug> =
            ingredients.iter().map(IngredientSlug::from).collect();
        let search_igredients_set: HashSet<IngredientSlug> = ingredients.iter().cloned().collect();
        let facets: Vec<Facet> = ingredients.iter().map(Into::into).collect();
        let query = BooleanQuery::new_multiterms_query(
            facets
                .iter()
                .map(|facet| Term::from_facet(ingredient_slug_field, &facet))
                .collect(),
        );
        let top_docs_collector =
            TopDocs::with_limit(limit).tweak_score(move |segment_reader: &SegmentReader| {
                let mut ingredient_reader =
                    segment_reader.facet_reader(ingredient_slug_field).unwrap();
                let query_ords = get_query_ords(&facets, &ingredient_reader);
                let mut facet_ords_buffer = Vec::with_capacity(20);

                move |doc: DocId, original_score: Score| {
                    calculate_score(
                        doc,
                        original_score,
                        &mut ingredient_reader,
                        &mut facet_ords_buffer,
                        &query_ords,
                    )
                }
            });
        let searcher = self.recipes_reader.searcher();
        let top_docs: Vec<_> = searcher
            .search(&query, &top_docs_collector)?
            .iter()
            .map(|(score, doc_id)| {
                let document = searcher.doc(*doc_id).unwrap();
                let recipe_title = document.get_all(recipe_title_field)[0]
                    .text()
                    .unwrap()
                    .to_string();
                let recipe_url = document.get_all(recipe_url_field)[0]
                    .text()
                    .unwrap()
                    .to_string();

                // TODO: improve this
                let chef_name_parts = document.get_all(recipe_chef_name_field);

                let recipe_chef_name = if chef_name_parts.len() > 0 {
                    Some(chef_name_parts[0].text().unwrap().to_string())
                } else {
                    None
                };

                let ingredient_slugs: Vec<IngredientSlug> = document
                    .get_all(ingredient_slug_field)
                    .iter()
                    .map(|ingredient| match ingredient {
                        Value::Facet(value) => IngredientSlug::from(value),
                        _ => unreachable!(),
                    })
                    .collect();
                let ingredient_slugs_set: HashSet<_> = ingredient_slugs.iter().cloned().collect();
                let missing_ingredients: Vec<_> = ingredient_slugs_set
                    .difference(&search_igredients_set)
                    .cloned()
                    .collect();

                RecipeSearchResult {
                    score: *score,
                    document,
                    recipe_title,
                    recipe_url,
                    recipe_chef_name,
                    ingredient_slugs: ingredient_slugs.iter().map(Into::into).collect(),
                    missing_ingredients: missing_ingredients.iter().map(Into::into).collect(),
                }
            })
            .collect();

        Ok(top_docs)
    }

    pub fn ingredients_by_prefix(&self, prefix: &str) -> Result<(Vec<Ingredient>, usize)> {
        let name_field = self.ingredients_name;
        let slug_field = self.ingredients_slug;

        let searcher = self.ingredients_reader.searcher();
        let term = Term::from_field_text(self.ingredients_name, prefix);
        let query = FuzzyTermQuery::new_prefix(term, 0, true);
        let (top_docs, count) = searcher.search(&query, &(TopDocs::with_limit(20), Count))?;

        let top_docs: Vec<Ingredient> = top_docs
            .iter()
            .map(|(score, doc_id)| {
                let document = searcher.doc(*doc_id).unwrap();
                let name = document.get_all(name_field)[0].text().unwrap().to_string();
                let slug = document.get_all(slug_field)[0].text().unwrap().to_string();

                Ingredient {
                    score: *score,
                    name,
                    slug,
                }
            })
            .collect();

        Ok((top_docs, count))
    }
}

pub struct RecipeSearchResult {
    pub score: Score,
    pub document: Document, // TODO: something more useful
    pub recipe_title: String,
    pub recipe_url: String,
    pub recipe_chef_name: Option<String>,
    pub ingredient_slugs: Vec<String>,
    pub missing_ingredients: Vec<String>,
}

#[derive(Serialize)]
pub struct Ingredient {
    pub score: Score,
    pub name: String,
    pub slug: String,
}

fn get_query_ords(facets: &[Facet], ingredient_reader: &FacetReader) -> HashSet<u64> {
    let facet_dict = ingredient_reader.facet_dict();

    facets
        .iter()
        .filter_map(|key| facet_dict.term_ord(key.encoded_str()))
        .collect()
}

fn calculate_score(
    doc: DocId,
    original_score: Score,
    ingredient_reader: &mut FacetReader,
    facet_ords_buffer: &mut Vec<u64>,
    query_ords: &HashSet<u64>,
) -> Score {
    ingredient_reader.facet_ords(doc, facet_ords_buffer);
    let missing_ingredients = facet_ords_buffer
        .iter()
        .filter(|o| !query_ords.contains(o))
        .count();
    let tweak = 1.0 / 4_f32.powi(missing_ingredients as i32);

    let tweaked_score = original_score * tweak;
    /*
    if false && tweaked_score > 2.0 {
        let matching_ingredients = facet_ords.intersection(&query_ords).count();
        println!(
            "{} = {} * {}  : {} missing, {} matching",
            tweaked_score, original_score, tweak, missing_ingredients, matching_ingredients
        );
    }
    */
    tweaked_score
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct IngredientSlug(String);

impl std::fmt::Display for IngredientSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Facet> for IngredientSlug {
    fn from(facet: Facet) -> IngredientSlug {
        IngredientSlug::from(&facet)
    }
}

impl From<&Facet> for IngredientSlug {
    fn from(facet: &Facet) -> IngredientSlug {
        IngredientSlug(facet.to_path()[1].to_owned())
    }
}

impl From<&String> for IngredientSlug {
    fn from(slug: &String) -> IngredientSlug {
        IngredientSlug::from(slug.clone())
    }
}

impl From<String> for IngredientSlug {
    fn from(slug: String) -> IngredientSlug {
        IngredientSlug(slug)
    }
}

impl From<&str> for IngredientSlug {
    fn from(slug: &str) -> IngredientSlug {
        IngredientSlug(slug.to_owned())
    }
}

impl Into<Facet> for IngredientSlug {
    fn into(self) -> Facet {
        (&self).into()
    }
}

impl Into<Facet> for &IngredientSlug {
    fn into(self) -> Facet {
        Facet::from(&format!("/ingredient/{}", self))
    }
}

impl Into<String> for &IngredientSlug {
    fn into(self) -> String {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingredient_slug_from_strings() {
        let slug = IngredientSlug::from("recipe");
        assert_eq!(IngredientSlug::from("recipe"), slug);
        assert_eq!(
            IngredientSlug::from(Facet::from("/ingredient/recipe")),
            slug
        );
        let facet: Facet = slug.into();
        assert_eq!(facet, Facet::from("/ingredient/recipe"));
    }
}
