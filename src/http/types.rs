use std::collections::HashMap;
use actix_web::HttpResponse;

pub type HttpServiceQueryHashingMapOk = HashMap<String, Vec<Vec<u8>>>;
pub type HttpServiceQueryHashingMapErr = HttpResponse;