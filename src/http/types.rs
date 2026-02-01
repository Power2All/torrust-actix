use std::collections::HashMap;
use actix_web::HttpResponse;
use crate::common::common::QueryValues;

pub type HttpServiceQueryHashingMapOk = HashMap<String, QueryValues>;
pub type HttpServiceQueryHashingMapErr = HttpResponse;