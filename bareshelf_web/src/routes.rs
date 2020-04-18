use std::collections::HashSet;

use actix_session::Session;
use actix_web::{error, http, web, Error, HttpResponse, Responder};
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::json;

use bareshelf::RecipeSearchResult;

pub(crate) async fn status() -> impl Responder {
    HttpResponse::Ok().json(json!({"status": "ok"}))
}

pub(crate) async fn index(
    tera: web::Data<tera::Tera>,
    searcher: web::Data<bareshelf::Searcher>,
    session: Session,
) -> Result<HttpResponse, Error> {
    let ingredients = get_ingredients(&session)?;
    let mut ctx = tera::Context::new();
    ctx.insert("ingredients", &ingredients);

    if !ingredients.is_empty() {
        info!("Searching with ingredients: {:?}", ingredients);
        let recipes = searcher
            .recipes_by_ingredients(&ingredients, 10)
            .map_err(|_| error::ErrorInternalServerError("failed to search"))?;
        ctx.insert(
            "recipes",
            &recipes
                .into_iter()
                .map(Recipe::from)
                .collect::<Vec<Recipe>>(),
        );
    }

    render(tera, "index.html", Some(&ctx))
}

#[derive(Serialize)]
pub struct Recipe {
    score: f32,
    title: String,
    ingredients: Vec<Ingredient>,
}

impl From<RecipeSearchResult> for Recipe {
    fn from(recipe: RecipeSearchResult) -> Self {
        let missing: HashSet<_> = recipe.missing_ingredients.iter().collect();
        Recipe {
            score: recipe.score,
            title: recipe.recipe_title,
            ingredients: recipe
                .ingredient_slugs
                .iter()
                .map(|slug| Ingredient {
                    slug: slug.to_owned(),
                    is_missing: missing.contains(slug),
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub struct Ingredient {
    slug: String,
    is_missing: bool,
}

#[derive(Deserialize)]
pub struct AddIngredientForm {
    ingredient: String,
}

pub(crate) async fn add_ingredient(
    form: web::Form<AddIngredientForm>,
    session: Session,
) -> Result<HttpResponse, Error> {
    info!("ingredients: {:?}", form.ingredient);
    let mut ingredients = get_ingredients(&session)?;
    ingredients.push(form.ingredient.to_owned());
    set_ingredients(&session, ingredients)?;

    Ok(found("/"))
}

fn get_ingredients(session: &Session) -> Result<Vec<String>, Error> {
    Ok(session
        .get("ingredients")
        .map_err(|_| error::ErrorInternalServerError("invalid ingredients list"))?
        .unwrap_or_default())
}

fn set_ingredients(session: &Session, ingredients: Vec<String>) -> Result<(), Error> {
    session
        .set("ingredients", ingredients)
        .map_err(|_| error::ErrorInternalServerError("failed to set ingredients"))
}

fn render(
    tmpl: web::Data<tera::Tera>,
    template_name: &str,
    context: Option<&tera::Context>,
) -> Result<HttpResponse, Error> {
    let body = tmpl
        .render(template_name, context.unwrap_or(&tera::Context::new()))
        .map_err(|_| error::ErrorInternalServerError("template errror"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

pub(crate) fn found<B>(location: &str) -> HttpResponse<B> {
    HttpResponse::Found()
        .header(http::header::LOCATION, location)
        .finish()
        .into_body()
}
