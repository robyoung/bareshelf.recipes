use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use tantivy::{
    schema::{Facet, Field, Schema, Value},
    Document,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub title: String,
    pub slug: String,
    pub url: String,
    pub chef_name: Option<String>,
    pub ingredients: Vec<Ingredient>,
}

impl Recipe {
    pub fn new(
        title: &str,
        slug: &str,
        url: &str,
        chef_name: Option<&str>,
        ingredients: Vec<Ingredient>,
    ) -> Self {
        Self {
            title: String::from(title),
            slug: String::from(slug),
            url: String::from(url),
            chef_name: chef_name.map(String::from),
            ingredients,
        }
    }

    pub(crate) fn from_doc(schema: &Schema, doc: &Document) -> Option<Self> {
        Some(Self {
            title: get_first_text(&doc, schema.get_field("title").unwrap())?,
            slug: get_first_text(&doc, schema.get_field("slug").unwrap())?,
            url: get_first_text(&doc, schema.get_field("url").unwrap())?,
            chef_name: get_first_text(&doc, schema.get_field("chef_name").unwrap()),
            ingredients: doc
                .get_all(schema.get_field("ingredient_name").unwrap())
                .iter()
                .zip(
                    doc.get_all(schema.get_field("ingredient_slug").unwrap())
                        .iter(),
                )
                .map(|(name, slug)| Ingredient {
                    name: name.text().unwrap().to_string(),
                    slug: match slug {
                        Value::Facet(value) => IngredientSlug::from(value).into(),
                        _ => unreachable!(),
                    },
                })
                .collect(),
        })
    }
}

fn get_first_text(doc: &Document, field: Field) -> Option<String> {
    Some(doc.get_first(field)?.text().unwrap().to_string())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Ingredient {
    pub name: String,
    pub slug: String,
}

impl Ingredient {
    pub fn new(name: &str, slug: &str) -> Self {
        Self {
            name: String::from(name),
            slug: String::from(slug),
        }
    }
}

impl PartialOrd for Ingredient {
    fn partial_cmp(&self, other: &Ingredient) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ingredient {
    fn cmp(&self, other: &Ingredient) -> Ordering {
        self.slug.cmp(&other.slug)
    }
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

impl Into<String> for IngredientSlug {
    fn into(self) -> String {
        (&self).into()
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
