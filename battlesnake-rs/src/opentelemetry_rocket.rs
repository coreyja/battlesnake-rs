use rocket::fairing::{Info, Kind};
use rocket::http::{Status, StatusClass};
use rocket::request::{FromRequest, Outcome};
use rocket::{Data, Request, Response, Rocket, State};

use opentelemetry::trace::{Span, StatusCode, Tracer};
use opentelemetry::KeyValue;

pub struct OpenTelemetryFairing {
    pub tracer: Option<opentelemetry::sdk::trace::Tracer>,
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
        Ok(rocket.manage::<Option<opentelemetry::sdk::trace::Tracer>>(self.tracer.clone()))
    }

    fn on_request(&self, request: &mut Request, _data: &Data) {
        if let Some(tracer) = &self.tracer {
            let request_path = request.uri().path();
            let span = tracer.start(request_path);
            span.set_attribute(KeyValue::new("http.method", request.method().as_str()));
            span.set_attribute(KeyValue::new("http.path", request_path.to_owned()));
            request.local_cache(|| WrappedSpan(Some(span)));
        }
    }

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
pub struct TracingInner<'a, 'b> {
    pub span: &'a opentelemetry::sdk::trace::Span,
    pub tracer: &'b opentelemetry::sdk::trace::Tracer,
}
#[derive(Clone)]
pub struct Tracing<'a, 'b> {
    pub inner: Option<TracingInner<'a, 'b>>,
}

// Allows a route to access the time a request was initiated.
impl<'a, 'r> FromRequest<'a, 'r> for Tracing<'a, 'a> {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Tracing<'a, 'a>, ()> {
        let t = request.guard::<State<Option<opentelemetry::sdk::trace::Tracer>>>();
        match &request.local_cache(|| WrappedSpan(None)) {
            WrappedSpan(Some(span)) => {
                if let Some(route) = request.route() {
                    span.update_name(route.uri.path().to_string())
                }
                t.map(|t2| match t2.inner() {
                    Some(t3) => Tracing {
                        inner: Some(TracingInner { span, tracer: t3 }),
                    },
                    None => Tracing { inner: None },
                })
            }
            WrappedSpan(None) => Outcome::Success(Tracing { inner: None }),
        }
    }
}
