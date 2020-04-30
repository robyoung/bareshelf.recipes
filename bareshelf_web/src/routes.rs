use std::collections::HashSet;

use actix_session::Session;
use actix_web::{error, web, Error, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;

use crate::{
    flash::{FlashMessage, FlashResponse},
    shelf,
    shelf::Shelf,
    views::RecipeSearchResult,
    sharing::{encode_share_token, decode_share_token},
};

pub(crate) async fn status() -> impl Responder {
    HttpResponse::Ok().json(json!({"status": "ok"}))
}

pub(crate) async fn index(
    tera: web::Data<tera::Tera>,
    searcher: web::Data<bareshelf::Searcher>,
    shelf: Shelf,
    flash: FlashMessage,
) -> Result<HttpResponse, Error> {
    let ingredients = shelf.get_ingredients(&shelf::Bucket::Ingredients)?;
    let mut ctx = tera::Context::new();
    ctx.insert("ingredients", &ingredients);
    ctx.insert("flash", &flash.take());

    if !ingredients.is_empty() {
        let ingredients: Vec<_> = ingredients.iter().map(|i| i.slug.clone()).collect();
        let recipes = searcher
            .recipes_by_ingredients(&ingredients, &[], &[], 100) // TODO: move this to a config
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
    shelf: Shelf,
    flash: FlashMessage,
) -> Result<HttpResponse, Error> {
    let mut ctx = tera::Context::new();

    let ingredients = shelf.get_ingredients(&shelf::Bucket::Ingredients)?;
    ctx.insert("ingredients", &ingredients);
    let key_ingredients = shelf.get_ingredients(&shelf::Bucket::KeyIngredients)?;
    ctx.insert("key_ingredients", &key_ingredients);
    let banned_ingredients = shelf.get_ingredients(&shelf::Bucket::BannedIngredients)?;
    ctx.insert("banned_ingredients", &banned_ingredients);

    ctx.insert("flash", &flash.take());

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
    shelf: Shelf,
) -> Result<FlashResponse, Error> {
    let ingredient = searcher
        .ingredient_by_name(&form.ingredient)
        .map_err(|_| error::ErrorInternalServerError("search error"))?;

    let ingredient = if ingredient.is_some() {
        ingredient
    } else {
        let mut ingredients =
            get_ingredients_by_prefix(&shelf, searcher.as_ref(), &form.bucket, &form.ingredient)?;

        if ingredients.is_empty() {
            None
        } else {
            Some(ingredients.remove(0))
        }
    };

    let flash = if let Some(ingredient) = ingredient {
        if shelf.add_ingredient(&form.bucket, &ingredient)? {
            format!(
                "Added {} to your {}",
                ingredient.name,
                form.bucket.flash_name()
            )
        } else {
            format!(
                "{} is already in your {}",
                ingredient.name,
                form.bucket.flash_name()
            )
        }
    } else {
        format!("No ingredients found matching \"{}\"", form.ingredient)
    };

    Ok(FlashResponse::new(
        Some(flash),
        form.redirect.as_ref().unwrap_or(&"/".to_string()),
    ))
}

pub(crate) async fn remove_ingredient(
    form: web::Form<IngredientForm>,
    shelf: Shelf,
) -> Result<FlashResponse, Error> {
    let flash = if let Some(ingredient) = shelf.remove_ingredient(&form.bucket, &form.ingredient)? {
        Some(format!("Removed {} from your shelf", ingredient.name))
    } else {
        None
    };
    Ok(FlashResponse::new(
        flash,
        form.redirect.as_ref().unwrap_or(&"/".to_string()),
    ))
}

#[derive(Deserialize)]
pub struct Search {
    term: String,
    bucket: shelf::Bucket,
}

pub(crate) async fn ingredients(
    search: web::Query<Search>,
    searcher: web::Data<bareshelf::Searcher>,
    shelf: Shelf,
) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(get_ingredients_by_prefix(
        &shelf,
        searcher.as_ref(),
        &search.bucket,
        &search.term,
    )?))
}

#[derive(Deserialize)]
pub(crate) struct Share {
    token: Option<String>,
}

pub(crate) async fn share_shelf(
    tera: web::Data<tera::Tera>,
    shelf: Shelf,
    session: Session,
    share: web::Query<Share>,
    app_data: web::Data<crate::AppData>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let mut ctx = tera::Context::new();
    if let Some(ref token) = share.token {
        let uid = decode_share_token(&app_data.cookie_key, token)?;

        if shelf.uid() != uid {
            session
                .set("uid", uid)
                .map_err(|_| error::ErrorInternalServerError("failed to update shelf"))?;
            shelf
                .remove_all()
                .map_err(|_| error::ErrorInternalServerError("failed to clean up old shelf"))?;
            ctx.insert("imported", &true);
        } else {
            ctx.insert("imported", &false);
        }
    } else {
        let connection_info = req.connection_info();
        let token = encode_share_token(&app_data.cookie_key, shelf.uid())?;
        ctx.insert("token", &token);
        ctx.insert("share_url", &format!("{}://{}{}?token={}", connection_info.scheme(), connection_info.host(), req.uri().path(), token));
    }
    render(tera, "share-shelf.html", Some(&ctx))
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

// TODO: figure out a better way of handling this.
// Two options I can think of:
// 1. Replace or fix the autocomplete library so that we always get a slug rather than sometimes a
//    prefix
// 2. Create a web specific searcher type that handles how searching is done in the web UI
fn get_ingredients_by_prefix(
    shelf: &Shelf,
    searcher: &bareshelf::Searcher,
    bucket: &shelf::Bucket,
    prefix: &str,
) -> Result<Vec<bareshelf::Ingredient>, Error> {
    let existing_ingredients: HashSet<_> = shelf.get_ingredients(&bucket)?.into_iter().collect();
    let (ingredients, _) = searcher
        .ingredients_by_prefix(&prefix)
        .map_err(|_| error::ErrorInternalServerError("failed to search ingredients"))?;
    let ingredients: Vec<_> = ingredients
        .into_iter()
        .filter(|ingredient| !existing_ingredients.contains(ingredient))
        .collect();

    Ok(ingredients)
}
