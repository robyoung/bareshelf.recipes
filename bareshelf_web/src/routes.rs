use actix_session::Session;
use actix_web::{error, http, web, Error, HttpResponse, Responder};
use log::info;
use serde::Deserialize;
use serde_json::json;

use bareshelf::{Error as BareshelfError, Ingredient};

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
    ctx.insert("flash", &pop_flash(&session)?);

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
        let mut ingredients = get_ingredients(&session)?;
        if ingredients.iter().find(|&i| i == &ingredient).is_none() {
            set_flash(
                &session,
                &format!("Added {} to your shelf", ingredient.name),
            )?;
            ingredients.push(ingredient);
            ingredients.sort_unstable();
            set_ingredients(&session, ingredients)?;
        } else {
            set_flash(
                &session,
                &format!("{} is already in your shelf", ingredient.name),
            )?;
        }
    } else {
        set_flash(&session, &format!("No ingredients found matching \"{}\"", form.ingredient))?;
    }

    Ok(found("/"))
}

pub(crate) async fn remove_ingredient(
    form: web::Form<IngredientForm>,
    session: Session,
) -> Result<HttpResponse, Error> {
    let ingredients = get_ingredients(&session)?;
    let ingredient = ingredients.iter().find(|i| *i.slug == form.ingredient);
    if let Some(ingredient) = ingredient {
        set_flash(
            &session,
            &format!("Removed {} from your shelf", ingredient.name),
        )?;

        set_ingredients(
            &session,
            ingredients
                .into_iter()
                .filter(|i| *i.slug != form.ingredient)
                .collect(),
        )?;
    }

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
        .unwrap_or_else(|_| {
            session.remove("ingredients");
            None
        })
        .unwrap_or_default())
}

fn set_ingredients(session: &Session, ingredients: Vec<Ingredient>) -> Result<(), Error> {
    session
        .set("ingredients", ingredients)
        .map_err(|_| error::ErrorInternalServerError("failed to set ingredients"))
}

fn set_flash(session: &Session, message: &str) -> Result<(), Error> {
    session
        .set("flash", message)
        .map_err(|_| error::ErrorInternalServerError("failed to set flash"))
}

fn pop_flash(session: &Session) -> Result<Option<String>, Error> {
    let flash = session
        .get("flash")
        .map_err(|_| error::ErrorInternalServerError("failed to get flash"))?;
    if flash.is_some() {
        session.remove("flash");
    }
    Ok(flash)
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
