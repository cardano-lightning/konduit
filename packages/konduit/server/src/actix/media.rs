use minicbor::Encode;
use problem_details::ProblemDetail;
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum MediaType {
    Json,
    Cbor,
}

impl MediaType {
    pub fn content_type(&self) -> &'static str {
        match self {
            MediaType::Json => "application/json",
            MediaType::Cbor => "application/cbor",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Media {
    pub media_type: MediaType,
    pub status: u16,
    pub body: Vec<u8>,
}

impl Media {
    fn error() -> Media {
        Media {
            media_type: MediaType::Json,
            status: 500,
            body: b"{\"status\":500,\"detail\":\"internal serialization error\"}".to_vec(),
        }
    }
}

pub trait ToMedia {
    fn to_media(self, media: &MediaType) -> Media;
}

impl<T, E> ToMedia for Result<T, E>
where
    T: Serialize + Encode<()>,
    E: ProblemDetail + Serialize + Encode<()>,
{
    fn to_media(self, type_: &MediaType) -> Media {
        match self {
            // TODO: Is a hardcoded `200` good enough??
            Ok(val) => to_media(type_, 200, &val),
            Err(e) => {
                let pd = e.to_body();
                to_media(type_, pd.status, &pd)
            }
        }
    }
}

fn to_media<T: Serialize + Encode<()>>(type_: &MediaType, status: u16, val: &T) -> Media {
    match type_ {
        MediaType::Cbor => Media {
            media_type: type_.clone(),
            status,
            body: minicbor::to_vec(val).expect("infallible"),
        },
        MediaType::Json => match serde_json::to_vec(val) {
            Ok(body) => Media {
                media_type: type_.clone(),
                status,
                body,
            },
            Err(e) => {
                log::error!("serialization failed: {e}");
                Media::error()
            }
        },
    }
}

#[cfg(feature = "actix")]
impl From<Media> for actix_web::HttpResponse {
    fn from(m: Media) -> Self {
        actix_web::HttpResponse::build(
            actix_web::http::StatusCode::from_u16(m.status)
                .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
        )
        .content_type(m.media_type.content_type())
        .body(m.body)
    }
}
#[cfg(feature = "actix")]
pub fn pick_media_type(req: &actix_web::dev::ServiceRequest) -> MediaType {
    if let Some(accept) = req.headers().get(actix_web::http::header::ACCEPT)
        && accept == "application/json"
    {
        return MediaType::Json;
    } else if let Some(ct) = req.headers().get(actix_web::http::header::CONTENT_TYPE)
        && ct == "application/json"
    {
        return MediaType::Json;
    }
    MediaType::Cbor
}

#[cfg(feature = "actix")]
pub fn get_media_type(req: &actix_web::HttpRequest) -> MediaType {
    use actix_web::HttpMessage;
    req.extensions()
        .get::<MediaType>()
        .cloned()
        .unwrap_or(MediaType::Cbor)
}
