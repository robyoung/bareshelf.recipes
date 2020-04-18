use std::path::Path;

use actix_session::CookieSession;
use actix_web::{middleware::Logger, web, App, HttpServer};
use tera::Tera;

pub mod routes;

pub async fn run_server() -> std::io::Result<()> {
    let cookie_key =
        base64::decode(&std::env::var("COOKIE_SECRET").expect("COOKIE_SECRET is required"))
            .expect("COOKIE_SECRET is not valid base64");
    let mut tera = Tera::new("/dev/null/*").unwrap();
    tera.add_raw_templates(vec![
        ("index.html", include_str!("../templates/index.html")),
        ("base.html", include_str!("../templates/base.html")),
    ])
    .unwrap();
    let searcher =
        bareshelf::searcher(Path::new("./search-index")).expect("Could not open search index");

    HttpServer::new(move || {
        let tera = tera.clone();
        let cookie_key = cookie_key.clone();
        let searcher = searcher.clone();

        App::new()
            .wrap(Logger::default())
            .wrap(
                CookieSession::signed(&cookie_key)
                    .name("glow")
                    .http_only(true)
                    .secure(false)
                    .max_age(60 * 60 * 24 * 3),
            )
            .data(tera)
            .data(searcher)
            .service(web::resource("/status").route(web::get().to(routes::status)))
            .service(
                web::scope("/")
                    .route("", web::get().to(routes::index))
                    .route("/add-ingredient", web::post().to(routes::add_ingredient)),
            )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
