use actix_session::SessionExt;
use actix_web::body::BoxBody;
use actix_web::{dev::Payload, FromRequest, HttpRequest, HttpResponse};
use actix_web::{error, Error, Responder};
use futures::future::{err, ok, Ready};

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
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse {
        let session = req.get_session();
        let set_flash = session
            .insert("flash", self.message)
            .map_err(|_| error::ErrorInternalServerError("failed to set flash"));
        if let Err(err) = set_flash {
            return HttpResponse::from(err);
        }

        let responder = HttpResponse::SeeOther()
            .append_header((actix_web::http::header::LOCATION, self.location))
            .finish();

        responder.respond_to(req)
    }
}
