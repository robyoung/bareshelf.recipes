use actix_session::Session;
use actix_web::{error, http, web, Error, HttpResponse, Responder};
use log::info;
use serde::Deserialize;
use serde_json::json;

use bareshelf::Ingredient;

use crate::views::RecipeSearchResult;

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
        let ingredients: Vec<_> = ingredients.iter().map(|i| i.slug.clone()).collect();
        let recipes = searcher
            .recipes_by_ingredients(&ingredients, 100)
            .map_err(|_| error::ErrorInternalServerError("failed to search"))?;
        ctx.insert(
            "recipes",
            &recipes
                .into_iter()
                .map(RecipeSearchResult::from)
                .collect::<Vec<_>>(),
        );
    }

    render(tera, "index.html", Some(&ctx))
}

#[derive(Deserialize)]
pub struct IngredientForm {
    ingredient: String,
}

pub(crate) async fn add_ingredient(
    form: web::Form<IngredientForm>,
    searcher: web::Data<bareshelf::Searcher>,
    session: Session,
) -> Result<HttpResponse, Error> {
    // TODO: set a failure message in the flash
    let ingredient = searcher.ingredient_by_name(&form.ingredient)
        .map_err(|_| error::ErrorNotFound(format!("ingredient not found: {:?}", form.ingredient)))?;

    let mut ingredients = get_ingredients(&session)?;
    ingredients.push(ingredient);
    ingredients.sort_unstable();
    set_ingredients(&session, ingredients)?;

    Ok(found("/"))
}

pub(crate) async fn remove_ingredient(
    form: web::Form<IngredientForm>,
    session: Session,
) -> Result<HttpResponse, Error> {
    set_ingredients(
        &session,
        get_ingredients(&session)?
            .into_iter()
            .filter(|i| *i.slug != form.ingredient)
            .collect(),
    )?;

    Ok(found("/"))
}

#[derive(Deserialize)]
pub struct Search {
    term: String,
}

pub(crate) async fn ingredients(
    search: web::Query<Search>,
    searcher: web::Data<bareshelf::Searcher>,
) -> Result<HttpResponse, Error> {
    let (ingredients, _) = searcher
        .ingredients_by_prefix(&search.term)
        .map_err(|_| error::ErrorInternalServerError("failed to search ingredients"))?;
    Ok(HttpResponse::Ok().json(ingredients))
}

fn get_ingredients(session: &Session) -> Result<Vec<Ingredient>, Error> {
    Ok(session
        .get("ingredients")
        .unwrap_or_else(|_| {session.remove("ingredients"); None})
        .unwrap_or_default())
}

fn set_ingredients(session: &Session, ingredients: Vec<Ingredient>) -> Result<(), Error> {
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
        .map_err(|e| error::ErrorInternalServerError(format!("template errror: {:?}", e)))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

pub(crate) fn found<B>(location: &str) -> HttpResponse<B> {
    HttpResponse::Found()
        .header(http::header::LOCATION, location)
        .finish()
        .into_body()
}
