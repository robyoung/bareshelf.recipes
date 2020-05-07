use std::path::Path;

use actix_session::CookieSession;
use actix_web::{middleware::Logger, web, App, HttpServer};
use tera::{Tera, Result as TeraResult};

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
        ("ui2.html", include_str!("../templates/ui2.html")),
        ("ui3.html", include_str!("../templates/ui3.html")),
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
        },
        Err(err) => Err(err),
    }
}

#[cfg(not(feature = "embedded-templates"))]
fn templates() -> TeraResult<Tera> {
    Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"))
}

pub(crate) struct AppData {
    cookie_key: Vec<u8>,
}

pub async fn run_server() -> std::io::Result<()> {
    let cookie_key =
        base64::decode(&std::env::var("COOKIE_SECRET").expect("COOKIE_SECRET is required"))
            .expect("COOKIE_SECRET is not valid base64");
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
            .wrap(
                CookieSession::signed(&cookie_key)
                    .name("glow")
                    .http_only(true)
                    .secure(false)
                    .max_age(60 * 60 * 24 * 30),
            )
            .data(AppData { cookie_key })
            .data(tera)
            .data(searcher)
            .data(sled)
            .service(web::resource("/status").route(web::get().to(routes::status)))
            .service(
                web::scope("/")
                    .route("", web::get().to(routes::index))
                    .route("/ui2", web::get().to(routes::ui2))
                    .route("/ui3", web::get().to(routes::ui3))
                    .route("/ingredients", web::get().to(routes::ingredients))
                    .route("/add-ingredient", web::post().to(routes::add_ingredient))
                    .route("/remove-ingredient", web::post().to(routes::remove_ingredient))
                    .route("/share-shelf", web::get().to(routes::share_shelf))
                    .route("/api/ingredients", web::get().to(routes::api_ingredients))
            )
    })
    .bind(app_host)?
    .run()
    .await
}
