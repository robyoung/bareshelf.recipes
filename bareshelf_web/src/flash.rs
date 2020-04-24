use actix_session::Session;
use actix_web::{error, Error};

pub(crate) fn set_flash(session: &Session, message: &str) -> Result<(), Error> {
    session
        .set("flash", message)
        .map_err(|_| error::ErrorInternalServerError("failed to set flash"))
}

pub(crate) fn pop_flash(session: &Session) -> Result<Option<String>, Error> {
    let flash = session
        .get("flash")
        .map_err(|_| error::ErrorInternalServerError("failed to get flash"))?;
    if flash.is_some() {
        session.remove("flash");
    }
    Ok(flash)
}
