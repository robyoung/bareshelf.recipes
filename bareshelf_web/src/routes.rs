use actix_session::Session;
use actix_web::{error, http, web, Error, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;

use bareshelf::{Error as BareshelfError};

use crate::{views::RecipeSearchResult, shelf, flash::{set_flash, pop_flash}};

pub(crate) async fn status() -> impl Responder {
    HttpResponse::Ok().json(json!({"status": "ok"}))
}
pub(crate) async fn index(
    tera: web::Data<tera::Tera>,
    searcher: web::Data<bareshelf::Searcher>,
    session: Session,
) -> Result<HttpResponse, Error> {
    let ingredients = shelf::get_ingredients(&session, &shelf::Bucket::Ingredients)?;
    let mut ctx = tera::Context::new();
    ctx.insert("ingredients", &ingredients);
    ctx.insert("flash", &pop_flash(&session)?);

    if !ingredients.is_empty() {
        let ingredients: Vec<_> = ingredients.iter().map(|i| i.slug.clone()).collect();
        let recipes = searcher
            .recipes_by_ingredients(&ingredients, &vec![], &vec![], 100)
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

pub(crate) async fn ui2(
    tera: web::Data<tera::Tera>,
    searcher: web::Data<bareshelf::Searcher>,
    session: Session,
) -> Result<HttpResponse, Error> {
    let mut ctx = tera::Context::new();

    let ingredients = shelf::get_ingredients(&session, &shelf::Bucket::Ingredients)?;
    ctx.insert("ingredients", &ingredients);
    let key_ingredients = shelf::get_ingredients(&session, &shelf::Bucket::KeyIngredients)?;
    ctx.insert("key_ingredients", &key_ingredients);
    let banned_ingredients = shelf::get_ingredients(&session, &shelf::Bucket::BannedIngredients)?;
    ctx.insert("banned_ingredients", &banned_ingredients);

    ctx.insert("flash", &pop_flash(&session)?);

    if !ingredients.is_empty() {
        let ingredients: Vec<_> = ingredients.iter().map(|i| i.slug.clone()).collect();
        let key_ingredients: Vec<_> = key_ingredients.iter().map(|i| i.slug.clone()).collect();
        let banned_ingredients: Vec<_> =
            banned_ingredients.iter().map(|i| i.slug.clone()).collect();
        let recipes = searcher
            .recipes_by_ingredients(&ingredients, &key_ingredients, &banned_ingredients, 100)
            .map_err(|_| error::ErrorInternalServerError("failed to search"))?;
        ctx.insert(
            "recipes",
            &recipes
                .into_iter()
                .map(RecipeSearchResult::from)
                .collect::<Vec<_>>(),
        );
    }

    render(tera, "ui2.html", Some(&ctx))
}

#[derive(Deserialize)]
pub struct IngredientForm {
    ingredient: String,
    bucket: shelf::Bucket,
    redirect: Option<String>,
}

pub(crate) async fn add_ingredient(
    form: web::Form<IngredientForm>,
    searcher: web::Data<bareshelf::Searcher>,
    session: Session,
) -> Result<HttpResponse, Error> {
    let ingredient = searcher
        .ingredient_by_name(&form.ingredient)
        .or_else(|_| {
            let (mut ingredients, _) = searcher.ingredients_by_prefix(&form.ingredient)?;

            if ingredients.is_empty() {
                Ok(None)
            } else {
                Ok(Some(ingredients.remove(0)))
            }
        })
        .map_err(|_: BareshelfError| error::ErrorInternalServerError("search error"))?;

    if let Some(ingredient) = ingredient {
        shelf::add_ingredient(&session, &form.bucket, ingredient)?;
    } else {
        set_flash(
            &session,
            &format!("No ingredients found matching \"{}\"", form.ingredient),
        )?;
    }

    Ok(found(form.redirect.as_ref().unwrap_or(&"/".to_string())))
}

pub(crate) async fn remove_ingredient(
    form: web::Form<IngredientForm>,
    session: Session,
) -> Result<HttpResponse, Error> {
    let ingredients = shelf::get_ingredients(&session, &form.bucket)?;
    let ingredient = ingredients.iter().find(|i| *i.slug == form.ingredient);
    if let Some(ingredient) = ingredient {
        set_flash(
            &session,
            &format!("Removed {} from your shelf", ingredient.name),
        )?;

        shelf::set_ingredients(
            &session,
            &form.bucket,
            ingredients
                .into_iter()
                .filter(|i| *i.slug != form.ingredient)
                .collect(),
        )?;
    }

    Ok(found(form.redirect.as_ref().unwrap_or(&"/".to_string())))
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
