use std::cmp;

use actix_session::Session;
use actix_web::{error, web, Error, HttpRequest, HttpResponse, Responder};
use bareshelf::{IngredientQuery, RecipeQuery};
use rand::seq::SliceRandom;
use serde::Deserialize;
use serde_json::json;

use crate::{
    flash::{FlashMessage, FlashResponse},
    sharing::{decode_share_token, encode_share_token},
    shelf,
    shelf::{ingredient_slugs, Shelf},
    views::RecipeSearchResult,
};

/// Basic route with no dependencies to check the server is up
pub(crate) async fn status() -> impl Responder {
    HttpResponse::Ok().json(json!({"status": "ok"}))
}

/// Recipe listing UI
///
/// This UI separates ingredients management from recipe listing.
/// If the user's shelf is empty they will automatically be directed
/// towards ingredients management.
pub(crate) async fn index(
    tera: web::Data<tera::Tera>,
    searcher: web::Data<bareshelf::Searcher>,
    shelf: Shelf,
    flash: FlashMessage,
) -> Result<HttpResponse, Error> {
    let mut ctx = tera::Context::new();

    let ingredients = ingredient_slugs(&shelf.get_ingredients(&shelf::Bucket::Ingredients)?);

    ctx.insert("flash", &flash.take());

    if !ingredients.is_empty() {
        let query = RecipeQuery::default()
            .shelf_ingredients(&ingredients)
            .key_ingredients(&ingredient_slugs(
                &shelf.get_ingredients(&shelf::Bucket::KeyIngredients)?,
            ))
            .banned_ingredients(&ingredient_slugs(
                &shelf.get_ingredients(&shelf::Bucket::BannedIngredients)?,
            ));

        let recipes = searcher
            .recipes(query)
            .map_err(|_| error::ErrorInternalServerError("failed to search"))?;

        let mut can_make_now = recipes
            .can_make_now()
            .map(RecipeSearchResult::from)
            .collect::<Vec<_>>();

        let mut one_missing = recipes
            .one_missing()
            .map(RecipeSearchResult::from)
            .collect::<Vec<_>>();

        let mut more_missing = recipes
            .more_missing()
            .map(RecipeSearchResult::from)
            .collect::<Vec<_>>();

        let mut rng = rand::thread_rng();

        can_make_now.shuffle(&mut rng);
        one_missing.shuffle(&mut rng);
        more_missing.shuffle(&mut rng);

        ctx.insert("can_make_now", &can_make_now);
        ctx.insert("one_missing", &one_missing);
        ctx.insert("more_missing", &more_missing);

        let mut next_ingredients: Vec<(String, usize)> = recipes
            .next_ingredients()
            .into_iter()
            .map(|(slug, count)| (slug.into(), *count))
            .collect();

        if next_ingredients.len() > 0 {
            next_ingredients.sort_by_key(|(_, count)| *count);
            next_ingredients.reverse();
            ctx.insert(
                "next_ingredients",
                &next_ingredients[..cmp::min(20, next_ingredients.len())],
            );
        }
    }

    render(tera, "index.html", Some(&ctx))
}

pub(crate) async fn ingredients(
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
    let popular_ingredients = searcher
        .popular_ingredients(
            RecipeQuery::default()
                .shelf_ingredients(&ingredient_slugs(&ingredients))
                .key_ingredients(&ingredient_slugs(&key_ingredients))
                .banned_ingredients(&ingredient_slugs(&banned_ingredients))
                .limit(50),
        )
        .map_err(|_| error::ErrorInternalServerError("failed to execute query"))?;
    ctx.insert("popular_ingredients", &popular_ingredients);

    ctx.insert("flash", &flash.take());

    render(tera, "ingredients.html", Some(&ctx))
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
    let mut ingredients = searcher
        .ingredients(IngredientQuery::by_name(&form.ingredient))
        .map_err(|_| error::ErrorInternalServerError("search error"))?;

    let ingredient = if ingredients.is_empty() {
        let mut ingredients =
            get_ingredients_by_prefix(&shelf, searcher.as_ref(), &form.bucket, &form.ingredient)?;

        if ingredients.is_empty() {
            None
        } else {
            Some(ingredients.remove(0))
        }
    } else {
        Some(ingredients.remove(0))
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

pub(crate) async fn api_ingredients(
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
        ctx.insert(
            "share_url",
            &format!(
                "{}://{}{}?token={}",
                connection_info.scheme(),
                connection_info.host(),
                req.uri().path(),
                token
            ),
        );
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
    let existing_ingredients = shelf.get_ingredients(&bucket)?;

    let query = IngredientQuery::by_prefix(&prefix).excluding(&existing_ingredients);

    let ingredients = searcher
        .ingredients(query)
        .map_err(|_| error::ErrorInternalServerError("failed to search ingredients"))?;

    Ok(ingredients)
}
