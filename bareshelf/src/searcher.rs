use std::{cmp::Ordering, collections::HashSet};

use tantivy::{
    collector::{Count, FacetCollector, TopDocs},
    fastfield::FacetReader,
    query::{AllQuery, BooleanQuery, FuzzyTermQuery, Occur, Query, QueryParser},
    schema::{Facet, Field, FieldType, Schema, Term},
    tokenizer::Token,
    DocAddress, DocId, Document, IndexReader, LeasedItem, Score, SegmentReader,
};

use crate::{
    datatypes::{Ingredient, IngredientSlug, Recipe},
    error::Result,
};

#[derive(Clone)]
pub struct Searcher {
    recipes_reader: IndexReader,
    recipes_schema: Schema,

    ingredients_index: tantivy::Index,
    ingredients_reader: IndexReader,
    ingredients_schema: Schema,
}

impl Searcher {
    pub(crate) fn new(recipes: &tantivy::Index, ingredients: &tantivy::Index) -> Result<Searcher> {
        Ok(Searcher {
            recipes_reader: recipes.reader()?,
            recipes_schema: recipes.schema(),

            ingredients_index: ingredients.clone(),
            ingredients_reader: ingredients.reader()?,
            ingredients_schema: ingredients.schema(),
        })
    }

    pub fn recipe_ingredients(&self) -> Result<Vec<(String, u64)>> {
        let searcher = self.recipes_reader.searcher();
        let mut facet_collector =
            FacetCollector::for_field(self.recipes_schema.get_field("ingredient_slug").unwrap());
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
        let ingredient_slug_field = self.recipes_schema.get_field("ingredient_slug").unwrap();

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

                let recipe = Recipe::from_doc(&self.recipes_schema, &document).unwrap();
                let ingredient_slugs_set: HashSet<_> = recipe
                    .ingredients
                    .iter()
                    .map(|i| IngredientSlug::from(&i.slug))
                    .collect();
                let missing_ingredients: Vec<_> = ingredient_slugs_set
                    .difference(&search_igredients_set)
                    .cloned()
                    .collect();
                RecipeSearchResult {
                    score: *score,
                    document,
                    recipe,
                    missing_ingredients: missing_ingredients.iter().map(Into::into).collect(),
                }
            })
            .collect();

        Ok(top_docs)
    }

    pub fn ingredients_by_prefix(&self, prefix: &str) -> Result<(Vec<Ingredient>, usize)> {
        let name_field = self.ingredients_schema.get_field("name").unwrap();
        let tokens = get_field_tokens(
            &self.ingredients_index,
            &self.ingredients_schema,
            name_field,
            prefix,
        )
        .unwrap();
        let query = BooleanQuery::from(
            tokens
                .iter()
                .map(|token| {
                    println!("token: {}", token.text);
                    let query: Box<dyn Query> = Box::new(FuzzyTermQuery::new_prefix(
                        Term::from_field_text(name_field, &token.text),
                        0,
                        true,
                    ));
                    (Occur::Must, query)
                })
                .collect::<Vec<_>>(),
        );

        let searcher = self.ingredients_reader.searcher();
        let (top_docs, count) = searcher.search(&query, &(TopDocs::with_limit(20), Count))?;

        let mut top_docs = self
            .load_ingredients(&searcher, top_docs)
            .into_iter()
            .map(|(_, i)| i)
            .collect::<Vec<_>>();

        // Use the same sorting as used by the materialize autocomplete
        top_docs.sort_by(|left, right| {
            let order = left
                .name
                .to_lowercase()
                .find(&tokens[0].text)
                .cmp(&right.name.to_lowercase().find(&tokens[0].text));
            if order == Ordering::Equal {
                left.name.len().cmp(&right.name.len())
            } else {
                order
            }
        });

        Ok((top_docs, count))
    }

    pub fn ingredient_by_name(&self, name: &str) -> Result<Option<Ingredient>> {
        let name_field = self.ingredients_schema.get_field("name").unwrap();
        let searcher = self.ingredients_reader.searcher();
        let query = QueryParser::for_index(&self.ingredients_index, vec![name_field])
            .parse_query(name)
            .unwrap();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(5))?;

        let top_docs = self.load_ingredients(&searcher, top_docs);

        Ok(top_docs
            .iter()
            .cloned()
            .map(|(_, i)| i)
            .find(|i| i.name.to_lowercase() == name.to_lowercase()))
    }

    fn load_ingredients(
        &self,
        searcher: &LeasedItem<tantivy::Searcher>,
        top_docs: Vec<(Score, DocAddress)>,
    ) -> Vec<(Score, Ingredient)> {
        let name_field = self.ingredients_schema.get_field("name").unwrap();
        let slug_field = self.ingredients_schema.get_field("slug").unwrap();

        top_docs
            .iter()
            .map(|(score, doc_id)| {
                let document = searcher.doc(*doc_id).unwrap();
                let name = document.get_all(name_field)[0].text().unwrap().to_string();
                let slug = document.get_all(slug_field)[0].text().unwrap().to_string();

                (*score, Ingredient::new(&name, &slug))
            })
            .collect()
    }
}

fn get_field_tokens(
    index: &tantivy::Index,
    schema: &Schema,
    field: Field,
    input: &str,
) -> Option<Vec<Token>> {
    let entry = schema.get_field_entry(field);
    match entry.field_type() {
        FieldType::Str(ref str_options) => {
            if let Some(options) = str_options.get_indexing_options() {
                let analyzer = index.tokenizers().get(options.tokenizer())?;
                let mut token_stream = analyzer.token_stream(input);
                let mut tokens = vec![];
                while let Some(token) = token_stream.next() {
                    tokens.push(token.clone());
                }
                Some(tokens)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub struct RecipeSearchResult {
    pub score: Score,
    pub document: Document, // TODO: something more useful
    pub recipe: Recipe,
    pub missing_ingredients: Vec<String>,
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

    original_score * tweak
}
