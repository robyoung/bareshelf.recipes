use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use log::error;
use tantivy::{
    collector::{Collector, FacetCollector, TopDocs},
    fastfield::FacetReader,
    query::{AllQuery, BooleanQuery, FuzzyTermQuery, Occur, Query, QueryParser, TermQuery},
    schema::{Facet, Field, FieldType, IndexRecordOption, Schema, Term},
    tokenizer::Token,
    DocAddress, DocId, IndexReader, LeasedItem, Score, SegmentReader,
};

use crate::{
    datatypes::{Ingredient, IngredientSlug, Recipe},
    error::Result,
    next_ingredient::NextIngredientCollector,
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

    // TODO: remove or roll in to recipe search
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

    /// Popular ingredients that the user does not already have
    pub fn popular_ingredients(&self, query: RecipeQuery) -> Result<Vec<(Ingredient, u64)>> {
        // query for all ingredients
        let searcher = self.recipes_reader.searcher();
        let mut facet_collector =
            FacetCollector::for_field(self.recipes_schema.get_field("ingredient_slug").unwrap());
        facet_collector.add_facet("/ingredient");
        let facet_counts = searcher.search(&AllQuery, &facet_collector)?;

        // HashSet of all ingredients in the RecipeQuery
        let query_ingredients = query
            .shelf_ingredients
            .into_iter()
            .chain(query.key_ingredients.into_iter())
            .chain(query.banned_ingredients.into_iter())
            .collect::<HashSet<IngredientSlug>>();

        // get facet counts for all ingredients on in the RecipeQuery
        let mut result: Vec<(IngredientSlug, u64)> = facet_counts
            .get("/ingredient")
            .map(|(facet, count)| (IngredientSlug::from(facet), count))
            .filter(|(facet, _)| !query_ingredients.contains(facet))
            .collect();

        result.sort_unstable_by_key(|(_, count)| std::cmp::Reverse(*count));

        // only grab the top N
        let results = result.into_iter().take(query.limit).collect::<Vec<_>>();

        // then query for the full Ingredient items
        let ingredients = self.ingredients(
            IngredientQuery::by_slugs(
                &results
                    .iter()
                    .cloned()
                    .map(|(slug, _)| slug)
                    .collect::<Vec<_>>(),
            )
            .with_limit(query.limit),
        )?;

        let mut ingredients = ingredients
            .into_iter()
            .map(|ingredient| (IngredientSlug::from(ingredient.slug.clone()), ingredient))
            .collect::<HashMap<_, _>>();

        Ok(results
            .into_iter()
            .map(|(slug, count)| {
                if !ingredients.contains_key(&slug) {
                    error!("could not find {:?}", slug);
                }
                (ingredients.remove(&slug).unwrap(), count)
            })
            .collect())
    }

    pub fn recipes(&self, query: RecipeQuery) -> Result<RecipeSearchResults> {
        let ingredient_slug_field = self.recipes_schema.get_field("ingredient_slug").unwrap();

        let shelf_igredients_set: HashSet<IngredientSlug> =
            query.shelf_ingredients.iter().cloned().collect();
        let searcher = self.recipes_reader.searcher();

        let (recipes, next_ingredients) = searcher.search(
            &self.recipes_query(&query, ingredient_slug_field),
            &(
                self.recipes_doc_collector(&query, ingredient_slug_field),
                self.recipes_ingredients_collector(&query, ingredient_slug_field),
            ),
        )?;

        let recipes: Vec<_> = recipes
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
                    .difference(&shelf_igredients_set)
                    .cloned()
                    .collect();

                RecipeSearchResult {
                    score: *score,
                    recipe,
                    missing_ingredients: missing_ingredients.iter().map(Into::into).collect(),
                }
            })
            .collect();

        let next_ingredients = next_ingredients
            .iter()
            .map(|(facet, count)| (IngredientSlug::from(facet), *count))
            .collect();

        Ok(RecipeSearchResults::new(query, recipes, next_ingredients))
    }

    fn recipes_query(&self, query: &RecipeQuery, ingredient_slug_field: Field) -> BooleanQuery {
        let key_ingredient_query: Vec<(Occur, Box<dyn Query>)> = if query.key_ingredients.is_empty()
        {
            vec![]
        } else {
            vec![(
                Occur::Must,
                Box::new(BooleanQuery::from(
                    query
                        .key_ingredients
                        .iter()
                        .map(slug_to_query(ingredient_slug_field, Occur::Should))
                        .collect::<Vec<_>>(),
                )),
            )]
        };
        BooleanQuery::from(
            query
                .shelf_ingredients
                .iter()
                .map(slug_to_query(ingredient_slug_field, Occur::Should))
                .chain(key_ingredient_query.into_iter())
                .chain(
                    query
                        .banned_ingredients
                        .iter()
                        .map(slug_to_query(ingredient_slug_field, Occur::MustNot)),
                )
                .collect::<Vec<_>>(),
        )
    }

    fn recipes_doc_collector(
        &self,
        query: &RecipeQuery,
        ingredient_slug_field: Field,
    ) -> impl Collector<Fruit = Vec<(f32, DocAddress)>> {
        let ingredients_facets: Vec<Facet> =
            query.shelf_ingredients.iter().map(Into::into).collect();

        TopDocs::with_limit(query.limit).tweak_score(move |segment_reader: &SegmentReader| {
            let mut ingredient_reader = segment_reader.facet_reader(ingredient_slug_field).unwrap();
            let query_ords = get_query_ords(&ingredients_facets, &ingredient_reader);
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
        })
    }

    fn recipes_ingredients_collector(
        &self,
        query: &RecipeQuery,
        ingredient_slug_field: Field,
    ) -> impl Collector<Fruit = HashMap<Facet, usize>> {
        let ingredients_facets: Vec<Facet> =
            query.shelf_ingredients.iter().map(Into::into).collect();

        NextIngredientCollector::new(ingredient_slug_field, ingredients_facets)
    }

    pub fn ingredients(&self, query: IngredientQuery) -> Result<Vec<Ingredient>> {
        let searcher = self.ingredients_reader.searcher();
        let top_docs = searcher.search(
            &self.ingredients_query(&query),
            &TopDocs::with_limit(query.limit.unwrap_or(20)),
        )?;
        let top_docs = self
            .load_ingredients(&searcher, top_docs)
            .into_iter()
            .map(|(_, i)| i)
            .collect::<Vec<_>>();

        Ok(self.post_process_ingredients(&query, top_docs))
    }

    fn ingredients_query(&self, query: &IngredientQuery) -> Box<dyn Query> {
        let name_field = self.ingredients_schema.get_field("name").unwrap();
        let slug_field = self.ingredients_schema.get_field("slug").unwrap();

        match &query.by {
            IngredientQueryBy::Prefix(prefix) => {
                let tokens = get_field_tokens(
                    &self.ingredients_index,
                    &self.ingredients_schema,
                    name_field,
                    &prefix,
                )
                .unwrap();
                Box::new(BooleanQuery::from(
                    tokens
                        .iter()
                        .map(|token| {
                            let query: Box<dyn Query> = Box::new(FuzzyTermQuery::new_prefix(
                                Term::from_field_text(name_field, &token.text),
                                0,
                                true,
                            ));
                            (Occur::Must, query)
                        })
                        .collect::<Vec<_>>(),
                ))
            }
            IngredientQueryBy::Name(name) => Box::new(
                QueryParser::for_index(&self.ingredients_index, vec![name_field])
                    .parse_query(&name)
                    .unwrap(),
            ),
            IngredientQueryBy::Slugs(slugs) => {
                let term_queries: Vec<(Occur, Box<dyn Query>)> = slugs
                    .iter()
                    .map(|slug| {
                        let term_query: Box<dyn Query> = Box::new(TermQuery::new(
                            Term::from_field_text(slug_field, slug.inner()),
                            IndexRecordOption::Basic,
                        ));
                        (Occur::Should, term_query)
                    })
                    .collect();
                Box::new(BooleanQuery::from(term_queries))
            }
            IngredientQueryBy::All => Box::new(AllQuery),
        }
    }

    fn post_process_ingredients(
        &self,
        query: &IngredientQuery,
        mut top_docs: Vec<Ingredient>,
    ) -> Vec<Ingredient> {
        match &query.by {
            IngredientQueryBy::Prefix(prefix) => {
                let name_field = self.ingredients_schema.get_field("name").unwrap();
                let tokens = get_field_tokens(
                    &self.ingredients_index,
                    &self.ingredients_schema,
                    name_field,
                    &prefix,
                )
                .unwrap();
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
            }
            IngredientQueryBy::Name(name) => {
                top_docs = top_docs
                    .iter()
                    .cloned()
                    .filter(|i| i.name.to_lowercase() == name.to_lowercase())
                    .collect();
            }
            _ => {}
        }
        if let Some(excluding) = &query.excluding {
            top_docs = top_docs
                .into_iter()
                .filter(|ingredient| !excluding.contains(ingredient))
                .collect();
        }
        top_docs
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

/// Query for Recipes
pub struct RecipeQuery {
    limit: usize,
    shelf_ingredients: Vec<IngredientSlug>,
    key_ingredients: Vec<IngredientSlug>,
    banned_ingredients: Vec<IngredientSlug>,
}

impl Default for RecipeQuery {
    fn default() -> Self {
        Self {
            limit: 100,
            shelf_ingredients: vec![],
            key_ingredients: vec![],
            banned_ingredients: vec![],
        }
    }
}

impl RecipeQuery {
    /// Set the maximum number of recipes to return
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set which ingredients are in the shelf
    pub fn shelf_ingredients(mut self, ingredients: &[String]) -> Self {
        self.shelf_ingredients = ingredients.iter().map(IngredientSlug::from).collect();
        self
    }

    /// Set which ingredients are key
    ///
    /// A Recipe must contain these to be returned.
    pub fn key_ingredients(mut self, ingredients: &[String]) -> Self {
        self.key_ingredients = ingredients.iter().map(IngredientSlug::from).collect();
        self
    }

    /// Set which ingredients are banned
    ///
    /// A Recipe must not contain these to be returned.
    pub fn banned_ingredients(mut self, ingredients: &[String]) -> Self {
        self.banned_ingredients = ingredients.iter().map(IngredientSlug::from).collect();
        self
    }
}

fn slug_to_query(
    field: Field,
    occur: Occur,
) -> impl Fn(&IngredientSlug) -> (Occur, Box<dyn Query>) {
    move |slug| {
        (
            occur,
            Box::new(TermQuery::new(
                Term::from_facet(field, &slug.into()),
                IndexRecordOption::WithFreqs,
            )),
        )
    }
}

pub struct IngredientQuery<'a> {
    by: IngredientQueryBy<'a>,
    excluding: Option<HashSet<Ingredient>>,
    limit: Option<usize>,
}

enum IngredientQueryBy<'a> {
    Prefix(String),
    Name(String),
    Slugs(&'a [IngredientSlug]),
    All,
}

impl<'a> IngredientQuery<'a> {
    pub fn by_prefix(prefix: &str) -> Self {
        Self::by(IngredientQueryBy::Prefix(prefix.to_string()))
    }

    pub fn by_name(name: &str) -> Self {
        Self::by(IngredientQueryBy::Name(name.to_string()))
    }

    pub fn by_slugs(slugs: &'a [IngredientSlug]) -> Self {
        Self::by(IngredientQueryBy::Slugs(slugs))
    }

    pub fn all() -> Self {
        Self::by(IngredientQueryBy::All)
    }

    fn by(by: IngredientQueryBy<'a>) -> Self {
        Self {
            by,
            excluding: Default::default(),
            limit: Default::default(),
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn excluding(mut self, excluding: &[Ingredient]) -> Self {
        self.excluding = Some(excluding.iter().cloned().collect());
        self
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

pub struct RecipeSearchResults {
    #[allow(dead_code)]
    query: RecipeQuery,
    recipes: Vec<RecipeSearchResult>,
    next_ingredients: HashMap<IngredientSlug, usize>,
}

impl RecipeSearchResults {
    fn new(
        query: RecipeQuery,
        recipes: Vec<RecipeSearchResult>,
        next_ingredients: HashMap<IngredientSlug, usize>,
    ) -> Self {
        Self {
            query,
            recipes,
            next_ingredients,
        }
    }

    pub fn all(&self) -> &[RecipeSearchResult] {
        &self.recipes
    }

    pub fn can_make_now(&self) -> impl Iterator<Item = &RecipeSearchResult> {
        self.recipes
            .iter()
            .filter(|recipe| recipe.missing_ingredients.is_empty())
    }

    pub fn one_missing(&self) -> impl Iterator<Item = &RecipeSearchResult> {
        self.recipes
            .iter()
            .filter(|recipe| recipe.missing_ingredients.len() == 1)
    }

    pub fn more_missing(&self) -> impl Iterator<Item = &RecipeSearchResult> {
        self.recipes.iter().filter(|recipe| {
            recipe.missing_ingredients.len() > 1 && recipe.missing_ingredients.len() < 6
        })
    }

    pub fn next_ingredients(&self) -> &HashMap<IngredientSlug, usize> {
        &self.next_ingredients
    }
}

pub struct RecipeSearchResult {
    pub score: Score,
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

#[cfg(test)]
mod tests {
    use super::{Ingredient, IngredientQuery, RecipeQuery, Searcher};
    use crate::tests::{setup_indexes, setup_ingredients_index, setup_recipes_index};

    #[derive(Default, Debug)]
    struct IngredientMatcher {
        name: Option<String>,
        slug: Option<String>,
    }

    impl PartialEq<Ingredient> for IngredientMatcher {
        fn eq(&self, other: &Ingredient) -> bool {
            if let Some(name) = &self.name {
                if name != &other.name {
                    return false;
                }
            }

            if let Some(slug) = &self.slug {
                if slug != &other.slug {
                    return false;
                }
            }
            true
        }
    }

    #[test]
    fn popular_ingredients() {
        let (recipes_index, ingredients_index) = setup_indexes();
        let searcher = Searcher::new(&recipes_index, &ingredients_index).unwrap();

        let query = RecipeQuery::default()
            .shelf_ingredients(&["milk".to_string()])
            .banned_ingredients(&["mushroom".to_string()])
            .limit(2);

        let result = searcher.popular_ingredients(query).unwrap();

        println!("{:?}", result);

        assert_eq!(
            IngredientMatcher {
                slug: Some(String::from("egg")),
                ..Default::default()
            },
            result[0].0,
        );
        assert_eq!(
            IngredientMatcher {
                slug: Some(String::from("oil")),
                ..Default::default()
            },
            result[1].0,
        );

        assert_eq!(3, result[0].1);
        assert_eq!(2, result[1].1);
    }

    #[test]
    fn tweak_score_with_facets() {
        let (recipes_index, ingredients_index) = setup_recipes_index();

        let searcher = Searcher::new(&recipes_index, &ingredients_index).unwrap();

        let query = RecipeQuery::default()
            .shelf_ingredients(&[
                "egg".to_string(),
                "oil".to_string(),
                "garlic".to_string(),
                "mushroom".to_string(),
            ])
            .limit(2);

        let results = searcher.recipes(query).unwrap();

        assert_eq!(
            results
                .all()
                .iter()
                .map(|r| r.recipe.title.to_owned())
                .collect::<Vec<String>>(),
            vec!["Fried egg", "Egg rolls"]
        );
    }

    #[test]
    fn one_of_key_ingredients_must_appear() {
        let (recipes_index, ingredients_index) = setup_recipes_index();

        let searcher = Searcher::new(&recipes_index, &ingredients_index).unwrap();

        let query = RecipeQuery::default()
            .shelf_ingredients(&["egg".to_string()])
            .key_ingredients(&["milk".to_string(), "garlic".to_string()]);

        let results = searcher.recipes(query).unwrap();

        assert_eq!(results.all().len(), 2);
    }

    #[test]
    fn ingredients_by_name() {
        let (recipes_index, ingredients_index) = setup_ingredients_index();

        let searcher = Searcher::new(&recipes_index, &ingredients_index).unwrap();
        let query = IngredientQuery::by_name("Sugar");
        let ingredients = searcher.ingredients(query).unwrap();

        assert_eq!(ingredients[0].name, "Sugar");
        assert_eq!(ingredients.len(), 1);
    }

    #[test]
    fn ingredients_by_prefix() {
        let (recipes_index, ingredients_index) = setup_ingredients_index();

        let searcher = Searcher::new(&recipes_index, &ingredients_index).unwrap();
        let query = IngredientQuery::by_prefix("brown su");
        let ingredients = searcher.ingredients(query).unwrap();

        assert_eq!(ingredients[0].name, "Brown sugar");
    }

    #[test]
    fn ingredients_by_prefix_butter() {
        let (recipes_index, ingredients_index) = setup_ingredients_index();

        let searcher = Searcher::new(&recipes_index, &ingredients_index).unwrap();
        let query = IngredientQuery::by_prefix("butt");
        let ingredients = searcher.ingredients(query).unwrap();

        assert_eq!(ingredients[0].name, "Butter");
    }
}
