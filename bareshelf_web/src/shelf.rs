use actix_session::Session;
use actix_web::{error, Error};
use serde::Deserialize;

use bareshelf::Ingredient;

use crate::flash::set_flash;

pub(crate) fn add_ingredient(
    session: &Session,
    bucket: &Bucket,
    ingredient: Ingredient,
) -> Result<(), Error> {
    let mut ingredients = get_ingredients(session, bucket)?;
    if ingredients.iter().find(|&i| i == &ingredient).is_none() {
        set_flash(
            session,
            &format!("Added {} to your {}", ingredient.name, bucket.flash_name()),
        )?;
        ingredients.push(ingredient);
        ingredients.sort_unstable();
        set_ingredients(session, bucket, ingredients)?;
    } else {
        set_flash(
            session,
            &format!(
                "{} is already in your {}",
                ingredient.name,
                bucket.flash_name()
            ),
        )?;
    }
    Ok(())
}

pub(crate) fn get_ingredients(
    session: &Session,
    bucket: &Bucket,
) -> Result<Vec<Ingredient>, Error> {
    Ok(session
        .get(&bucket.session_key())
        .unwrap_or_else(|_| {
            session.remove(&bucket.session_key());
            None
        })
        .unwrap_or_default())
}

pub(crate) fn set_ingredients(
    session: &Session,
    bucket: &Bucket,
    ingredients: Vec<Ingredient>,
) -> Result<(), Error> {
    session
        .set(&bucket.session_key(), ingredients)
        .map_err(|_| error::ErrorInternalServerError("failed to set ingredients"))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Bucket {
    KeyIngredients,
    BannedIngredients,
    Ingredients,
}

impl Bucket {
    pub(crate) fn flash_name(&self) -> String {
        match self {
            Bucket::KeyIngredients => "key ingredients".to_string(),
            Bucket::BannedIngredients => "banned ingredients".to_string(),
            Bucket::Ingredients => "shelf".to_string(),
        }
    }

    /// The key to use in the session object
    fn session_key(&self) -> String {
        match self {
            Bucket::KeyIngredients => "key_ingredients".to_string(),
            Bucket::BannedIngredients => "banned_ingredients".to_string(),
            Bucket::Ingredients => "ingredients".to_string(),
        }
    }
}
