use std::path::Path;

use pyo3::create_exception;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

use bareshelf::{indexer, Indexer};
use bareshelf::{Ingredient as BareshelfIngredient, Recipe as BareshelfRecipe};

#[pyclass]
struct Index {
    indexer: Indexer,
}

#[pymethods]
impl Index {
    pub fn add_recipe(&mut self, recipe: Recipe) {
        self.indexer.add_recipe(recipe.into());
    }

    pub fn add_ingredient(&mut self, ingredient: Ingredient) {
        self.indexer.add_ingredient(ingredient.into());
    }

    pub fn commit(&mut self) -> PyResult<()> {
        if let Err(err) = self.indexer.commit() {
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
    url: String,
    chef_name: Option<String>,
    image_name: Option<String>,
    ingredients: Vec<Ingredient>,
}

impl From<Recipe> for BareshelfRecipe {
    fn from(recipe: Recipe) -> Self {
        BareshelfRecipe {
            title: recipe.title,
            slug: recipe.slug,
            url: recipe.url,
            chef_name: recipe.chef_name,
            image_name: recipe.image_name,
            ingredients: recipe.ingredients.iter().cloned().map(Into::into).collect(),
        }
    }
}

#[pymethods]
impl Recipe {
    #[new]
    fn new(title: String, slug: String, url: String) -> Self {
        Self {
            title,
            slug,
            url,
            chef_name: None,
            image_name: None,
            ingredients: vec![],
        }
    }

    #[setter]
    fn set_chef_name(&mut self, chef_name: String) {
        self.chef_name = Some(chef_name);
    }

    #[setter]
    fn set_image_name(&mut self, image_name: String) {
        self.image_name = Some(image_name);
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

impl From<Ingredient> for BareshelfIngredient {
    fn from(ingredient: Ingredient) -> Self {
        BareshelfIngredient {
            name: ingredient.name,
            slug: ingredient.slug,
        }
    }
}

#[pymethods]
impl Ingredient {
    #[new]
    fn new(name: String, slug: String) -> Self {
        Self { name, slug }
    }
}

#[pyfunction]
fn create_or_open(path: String) -> PyResult<Index> {
    let indexer = match indexer(Path::new(&path)) {
        Ok(indexer) => indexer,
        Err(err) => return Err(PyErr::new::<IndexError, _>(format!("{}", err))),
    };

    Ok(Index { indexer })
}

#[pymodule]
fn bareshelf_indexer(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(create_or_open))?;
    m.add_class::<Index>()?;
    m.add_class::<Recipe>()?;
    m.add_class::<Ingredient>()?;

    Ok(())
}

create_exception!(bareshelf_indexer, Error, pyo3::exceptions::PyException);
create_exception!(bareshelf_indexer, IndexError, Error);
