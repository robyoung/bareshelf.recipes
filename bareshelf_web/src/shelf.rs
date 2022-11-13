use actix_session::SessionExt;
use actix_web::{dev::Payload, web, FromRequest, HttpRequest};
use futures::future::{ok, Ready};
use rand::Rng;
use serde::Deserialize;

use bareshelf::Ingredient;

use crate::error::Error;

pub(crate) fn ingredient_slugs(ingredients: &[Ingredient]) -> Vec<String> {
    ingredients.iter().map(Ingredient::slug).collect()
}

pub(crate) struct Shelf {
    sled: sled::Db, // TODO: replace this with a trait if testing becomes slow
    uid: u32,
}

impl Shelf {
    pub(crate) fn add_ingredient(
        &self,
        bucket: &Bucket,
        ingredient: &Ingredient,
    ) -> Result<bool, Error> {
        let mut ingredients = self.get_ingredients(bucket)?;
        if !ingredients.iter().any(|i| i == ingredient) {
            ingredients.push(ingredient.clone());
            ingredients.sort_unstable();
            self.set_ingredients(bucket, ingredients)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn remove_ingredient(
        &self,
        bucket: &Bucket,
        slug: &str,
    ) -> Result<Option<Ingredient>, Error> {
        let ingredients = self.get_ingredients(bucket)?;
        let ingredient = ingredients.iter().find(|i| i.slug == slug);
        if let Some(ingredient) = ingredient {
            let ingredient = ingredient.clone();
            self.set_ingredients(
                bucket,
                ingredients.into_iter().filter(|i| i.slug != slug).collect(),
            )?;
            Ok(Some(ingredient))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn get_ingredients(&self, bucket: &Bucket) -> Result<Vec<Ingredient>, Error> {
        let result = self.sled.get(self.key(&bucket.session_key()).as_bytes())?;

        if let Some(result) = result {
            Ok(serde_json::from_slice(&result)?)
        } else {
            Ok(vec![])
        }
    }

    pub(crate) fn remove_all(&self) -> Result<(), Error> {
        for bucket in [
            Bucket::KeyIngredients,
            Bucket::BannedIngredients,
            Bucket::Ingredients,
        ]
        .iter()
        {
            self.sled.remove(self.key(&bucket.session_key()))?;
        }
        Ok(())
    }

    pub(crate) fn uid(&self) -> u32 {
        self.uid
    }

    fn set_ingredients(&self, bucket: &Bucket, ingredients: Vec<Ingredient>) -> Result<(), Error> {
        self.sled.insert(
            self.key(&bucket.session_key()).as_bytes(),
            serde_json::to_vec(&ingredients)?,
        )?;
        Ok(())
    }

    fn key(&self, path: &str) -> String {
        format!("/{}/{}", self.uid, path)
    }
}

impl FromRequest for Shelf {
    type Error = Error;
    type Future = Ready<Result<Shelf, Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let session = req.get_session();
        let uid = session.get("uid").unwrap_or(None).unwrap_or_else(|| {
            let mut rng = rand::thread_rng();
            let uid = rng.gen::<u32>();
            session.insert("uid", uid).unwrap();
            uid
        });
        let sled = req.app_data::<web::Data<sled::Db>>().unwrap();
        ok(Shelf {
            sled: sled.get_ref().clone(),
            uid,
        })
    }
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
