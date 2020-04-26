use actix_session::UserSession;
use actix_web::{dev::Payload, FromRequest, HttpRequest, HttpResponse};
use actix_web::{error, Error, Responder};
use futures::future::{err, ok, Ready, LocalBoxFuture, TryFutureExt};


#[derive(Debug)]
pub(crate) struct FlashMessage(Option<String>);

impl FlashMessage {
    pub fn take(self) -> Option<String> {
        self.0
    }
}

impl FromRequest for FlashMessage {
    type Error = Error;
    type Future = Ready<Result<FlashMessage, Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let session = req.get_session();
        if let Ok(flash) = session.get("flash") {
            session.remove("flash");
            ok(FlashMessage(flash))
        } else {
            err(error::ErrorBadRequest("Unable to read flash message"))
        }
    }
}

pub(crate) struct FlashResponse {
    message: Option<String>,
    location: String,
}

impl FlashResponse {
    pub fn new(message: Option<String>, location: &str) -> Self {
        FlashResponse {
            message,
            location: location.to_owned(),
        }
    }
}

impl Responder for FlashResponse {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<HttpResponse, Self::Error>>;

    fn respond_to(self, req: &HttpRequest) -> Self::Future {
        let session = req.get_session();
        let set_flash = session
            .set("flash", self.message)
            .map_err(|_| error::ErrorInternalServerError("failed to set flash"));

        let responder = HttpResponse::SeeOther()
            .header(actix_web::http::header::LOCATION, self.location)
            .finish();

        let out = responder.respond_to(req).err_into().and_then(|res| async {
            if set_flash.is_err() {
                Err(set_flash.err().unwrap())
            } else {
                Ok(res)
            }
        });

        Box::pin(out)
    }
}
