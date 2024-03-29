use std::path::Path;

use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, middleware::Logger, web, App, HttpServer};
use tera::{Result as TeraResult, Tera};

mod error;
mod flash;
mod routes;
mod sharing;
mod shelf;
mod views;

#[cfg(feature = "embedded-templates")]
fn templates() -> TeraResult<Tera> {
    let templates = vec![
        ("index.html", include_str!("../templates/index.html")),
        (
            "ingredients.html",
            include_str!("../templates/ingredients.html"),
        ),
        ("base.html", include_str!("../templates/base.html")),
        ("macros.html", include_str!("../templates/macros.html")),
        (
            "includes/nav-links.html",
            include_str!("../templates/includes/nav-links.html"),
        ),
        (
            "share-shelf.html",
            include_str!("../templates/share-shelf.html"),
        ),
    ];
    match Tera::new("/dev/null/*") {
        Ok(mut tera) => {
            tera.add_raw_templates(templates)?;
            Ok(tera)
        }
        Err(err) => Err(err),
    }
}

#[cfg(not(feature = "embedded-templates"))]
fn templates() -> TeraResult<Tera> {
    Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"))
}

pub(crate) struct AppData {
    cookie_key: Key,
}

pub async fn run_server() -> std::io::Result<()> {
    let cookie_key = Key::from(
        &base64::decode(&std::env::var("COOKIE_SECRET").expect("COOKIE_SECRET is required"))
            .expect("COOKIE_SECRET is not valid base64"),
    );
    let app_host = std::env::var("APP_HOST").expect("APP_HOST must be set");
    let tera = templates().unwrap();
    let searcher = bareshelf::searcher(Path::new(
        &std::env::var("SEARCH_INDEX_PATH").unwrap_or_else(|_| "./search-index".to_string()),
    ))
    .expect("Could not open search index");
    let sled =
        sled::open(&std::env::var("SLED_PATH").unwrap_or_else(|_| "./sled".to_string())).unwrap();

    HttpServer::new(move || {
        let tera = tera.clone();
        let cookie_key = cookie_key.clone();
        let searcher = searcher.clone();
        let sled = sled.clone();

        App::new()
            .wrap(Logger::default())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                cookie_key.clone(),
            ))
            .app_data(AppData { cookie_key })
            .app_data(tera)
            .app_data(searcher)
            .app_data(sled)
            .service(web::resource("/status").route(web::get().to(routes::status)))
            .service(
                web::scope("/")
                    .route("", web::get().to(routes::index))
                    .route("/ingredients", web::get().to(routes::ingredients))
                    .route("/add-ingredient", web::post().to(routes::add_ingredient))
                    .route(
                        "/remove-ingredient",
                        web::post().to(routes::remove_ingredient),
                    )
                    .route("/share-shelf", web::get().to(routes::share_shelf))
                    .route("/api/ingredients", web::get().to(routes::api_ingredients)),
            )
    })
    .bind(app_host)?
    .run()
    .await
}
