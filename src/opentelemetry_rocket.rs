use rocket::fairing::{Info, Kind};
use rocket::http::{Status, StatusClass};
use rocket::request::{FromRequest, Outcome};
use rocket::{Data, Request, Response, Rocket, State};

use opentelemetry::trace::{Span, StatusCode, Tracer};
use opentelemetry::KeyValue;

pub struct OpenTelemetryFairing {
    pub tracer: opentelemetry::sdk::trace::Tracer,
}

#[derive(Clone)]
struct WrappedSpan(Option<opentelemetry::sdk::trace::Span>);

impl rocket::fairing::Fairing for OpenTelemetryFairing {
    fn info(&self) -> Info {
        Info {
            name: "OpenTelemetry Fairing",
            kind: Kind::Request | Kind::Response | Kind::Attach,
        }
    }

    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        Ok(rocket.manage(self.tracer.clone()))
    }

    /// Stores the start time of the request in request-local state.
    fn on_request(&self, request: &mut Request, _data: &Data) {
        let request_path = request.uri().path();
        let span = self.tracer.start(request_path);
        span.set_attribute(KeyValue::new("http.method", request.method().as_str()));
        span.set_attribute(KeyValue::new("http.path", request_path.to_owned()));
        request.local_cache(|| WrappedSpan(Some(span)));
    }

    /// Adds a header to the response indicating how long the server took to
    /// process the request.
    fn on_response(&self, request: &Request, response: &mut Response) {
        let wrapped_span = request.local_cache(|| WrappedSpan(None));
        if let Some(span) = &wrapped_span.0 {
            let span_status = match response.status().class() {
                StatusClass::ClientError => StatusCode::Error,
                StatusClass::ServerError => StatusCode::Error,
                _ => StatusCode::Ok,
            };
            span.set_status(span_status, response.status().reason.to_string());
            span.set_attribute(KeyValue::new(
                "http.status_code",
                response.status().code.to_string(),
            ));
            span.end();
        }
    }
}

/// Request guard used to retrieve the start time of a request.
#[derive(Clone)]
pub struct Tracing<'a, 'b> {
    pub span: &'a opentelemetry::sdk::trace::Span,
    pub tracer: &'b opentelemetry::sdk::trace::Tracer,
}

// Allows a route to access the time a request was initiated.
impl<'a, 'r> FromRequest<'a, 'r> for Tracing<'a, 'a> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Tracing<'a, 'a>, ()> {
        let t = request.guard::<State<opentelemetry::sdk::trace::Tracer>>();
        match &request.local_cache(|| WrappedSpan(None)) {
            WrappedSpan(Some(span)) => {
                if let Some(route) = request.route() {
                    span.update_name(route.uri.path().to_string())
                }
                t.map(|tracer| Tracing {
                    span,
                    tracer: tracer.inner(),
                })
            }
            WrappedSpan(None) => Outcome::Failure((Status::InternalServerError, ())),
        }
    }
}
