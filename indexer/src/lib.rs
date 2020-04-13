use std::path::Path;

use pyo3::create_exception;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use tantivy::directory::MmapDirectory;
use tantivy::schema::{Schema, STORED};

fn create_schema() -> Schema {
    let mut schema_builder = tantivy::schema::Schema::builder();
    schema_builder.add_text_field("recipe_title", STORED);
    schema_builder.add_text_field("recipe_slug", STORED);
    schema_builder.add_facet_field("ingredient_slug");
    schema_builder.add_text_field("ingredient_name", STORED);
    schema_builder.build()
}

#[pyclass]
struct Index {
    writer: tantivy::IndexWriter,
    recipe_title: tantivy::schema::Field,
    recipe_slug: tantivy::schema::Field,
    ingredient_name: tantivy::schema::Field,
    ingredient_slug: tantivy::schema::Field,
}

#[pymethods]
impl Index {
    pub fn add(&mut self, recipe: Recipe) {
        let mut document = tantivy::schema::Document::default();
        document.add_text(self.recipe_title, &recipe.title);
        document.add_text(self.recipe_slug, &recipe.slug);
        for ingredient in recipe.ingredients {
            document.add_facet(
                self.ingredient_slug,
                &format!("/ingredient/{}", ingredient.slug),
            );
            document.add_text(self.ingredient_name, &ingredient.name);
        }
        self.writer.add_document(document);
    }

    pub fn commit(&mut self) -> PyResult<()> {
        if let Err(err) = self.writer.commit() {
            Err(PyErr::new::<IndexError, _>(format!("{}", err)))
        } else {
            Ok(())
        }
    }
}

#[pyclass]
#[derive(Clone)]
struct Recipe {
    title: String,
    slug: String,
    ingredients: Vec<Ingredient>,
}

#[pymethods]
impl Recipe {
    #[new]
    fn new(title: String, slug: String) -> Self {
        Self {
            title,
            slug,
            ingredients: vec![],
        }
    }

    pub fn add_ingredient(&mut self, name: String, slug: String) {
        self.ingredients.push(Ingredient { name, slug });
    }
}

#[pyclass]
#[derive(Clone)]
struct Ingredient {
    name: String,
    slug: String,
}

#[pyfunction]
fn create_or_open(path: String) -> PyResult<Index> {
    let schema = create_schema();

    let path = Path::new(&path);
    if !path.exists() || !path.is_dir() {
        return Err(PyErr::new::<IndexError, _>(
            "Path must exist and be a directory",
        ));
    }

    let directory = match MmapDirectory::open(path) {
        Ok(directory) => directory,
        Err(err) => return Err(PyErr::new::<IndexError, _>(format!("{}", err))),
    };
    let index = match tantivy::Index::open_or_create(directory, schema.clone()) {
        Ok(index) => index,
        Err(err) => return Err(PyErr::new::<IndexError, _>(format!("{}", err))),
    };
    let writer = match index.writer(30_000_000) {
        Ok(writer) => writer,
        Err(err) => return Err(PyErr::new::<IndexError, _>(format!("{}", err))),
    };

    Ok(Index {
        writer,
        recipe_title: schema.get_field("recipe_title").unwrap(),
        recipe_slug: schema.get_field("recipe_slug").unwrap(),
        ingredient_name: schema.get_field("ingredient_name").unwrap(),
        ingredient_slug: schema.get_field("ingredient_slug").unwrap(),
    })
}

#[pymodule]
fn indexer(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(create_or_open))?;
    m.add_class::<Index>()?;
    m.add_class::<Recipe>()?;
    m.add_class::<Ingredient>()?;

    Ok(())
}

create_exception!(indexer, Error, pyo3::exceptions::Exception);
create_exception!(indexer, IndexError, Error);

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use tantivy::collector::TopDocs;
    use tantivy::doc;
    use tantivy::query::BooleanQuery;
    use tantivy::schema::*;
    use tantivy::{DocId, Index, Score, SegmentReader};

    #[test]
    fn tweak_score_with_facets() {
        let mut schema_builder = Schema::builder();

        let title = schema_builder.add_text_field("title", STORED);
        let ingredient = schema_builder.add_facet_field("ingredient");

        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema.clone());

        let mut index_writer = index.writer(30_000_000).unwrap();

        index_writer.add_document(doc!(
            title => "Fried egg",
            ingredient => Facet::from("/ingredient/egg"),
            ingredient => Facet::from("/ingredient/oil"),
        ));
        index_writer.add_document(doc!(
            title => "Scrambled egg",
            ingredient => Facet::from("/ingredient/egg"),
            ingredient => Facet::from("/ingredient/butter"),
            ingredient => Facet::from("/ingredient/milk"),
            ingredient => Facet::from("/ingredient/salt"),
        ));
        index_writer.add_document(doc!(
            title => "Egg rolls",
            ingredient => Facet::from("/ingredient/egg"),
            ingredient => Facet::from("/ingredient/garlic"),
            ingredient => Facet::from("/ingredient/salt"),
            ingredient => Facet::from("/ingredient/oil"),
            ingredient => Facet::from("/ingredient/tortilla-wrap"),
            ingredient => Facet::from("/ingredient/mushroom"),
        ));
        index_writer.commit().unwrap();

        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        {
            let facets = vec![
                Facet::from("/ingredient/egg"),
                Facet::from("/ingredient/oil"),
                Facet::from("/ingredient/garlic"),
                Facet::from("/ingredient/mushroom"),
            ];
            let query = BooleanQuery::new_multiterms_query(
                facets
                    .iter()
                    .map(|key| Term::from_facet(ingredient, &key))
                    .collect(),
            );
            let top_docs_by_custom_score =
                TopDocs::with_limit(2).tweak_score(move |segment_reader: &SegmentReader| {
                    let mut ingredient_reader = segment_reader.facet_reader(ingredient).unwrap();
                    let facet_dict = ingredient_reader.facet_dict();

                    let query_ords: HashSet<u64> = facets
                        .iter()
                        .filter_map(|key| facet_dict.term_ord(key.encoded_str()))
                        .collect();

                    let mut facet_ords_buffer: Vec<u64> = Vec::with_capacity(20);

                    move |doc: DocId, original_score: Score| {
                        ingredient_reader.facet_ords(doc, &mut facet_ords_buffer);
                        let missing_ingredients = facet_ords_buffer
                            .iter()
                            .cloned()
                            .collect::<HashSet<u64>>()
                            .difference(&query_ords)
                            .count();
                        let tweak = 1.0 / 4_f32.powi(missing_ingredients as i32);

                        original_score * tweak
                    }
                });
            let top_docs = searcher.search(&query, &top_docs_by_custom_score).unwrap();

            let titles: Vec<String> = top_docs
                .iter()
                .map(|(_, doc_id)| {
                    searcher
                        .doc(*doc_id)
                        .unwrap()
                        .get_first(title)
                        .unwrap()
                        .text()
                        .unwrap()
                        .to_owned()
                })
                .collect();
            assert_eq!(titles, vec!["Fried egg", "Egg rolls"]);
        }
    }
}
