use bareshelf_web::run_server;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    run_server().await
}
