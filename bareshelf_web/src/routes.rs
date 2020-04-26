use actix_web::{error, web, Error, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;

use bareshelf::Error as BareshelfError;

use crate::{
    flash::{FlashMessage, FlashResponse},
    shelf,
    shelf::Shelf,
    views::RecipeSearchResult,
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
        let (mut ingredients, _) = searcher
            .ingredients_by_prefix(&form.ingredient)
            .map_err(|_: BareshelfError| error::ErrorInternalServerError("search error"))?;

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
