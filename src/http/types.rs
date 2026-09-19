use actix_web::HttpResponse;
use crate::common::types::QueryMap;

pub type HttpServiceQueryHashingMapOk = QueryMap;
pub type HttpServiceQueryHashingMapErr = HttpResponse;