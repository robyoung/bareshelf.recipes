use std::path::Path;

use pyo3::create_exception;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use tantivy;
use tantivy::directory::MmapDirectory;
use tantivy::schema::STORED;

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
            document.add_facet(self.ingredient_slug, &format!("/{}", ingredient.slug));
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
    let mut schema_builder = tantivy::schema::Schema::builder();
    let recipe_title = schema_builder.add_text_field("recipe_title", STORED);
    let recipe_slug = schema_builder.add_text_field("recipe_slug", STORED);
    let ingredient_slug = schema_builder.add_facet_field("ingredient_slug");
    let ingredient_name = schema_builder.add_text_field("ingredient_name", STORED);
    let schema = schema_builder.build();

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
        recipe_title,
        recipe_slug,
        ingredient_name,
        ingredient_slug,
    })
}

#[pymodule]
fn search(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(create_or_open))?;
    m.add_class::<Index>()?;
    m.add_class::<Recipe>()?;
    m.add_class::<Ingredient>()?;

    Ok(())
}

create_exception!(search, Error, pyo3::exceptions::Exception);
create_exception!(search, IndexError, Error);
