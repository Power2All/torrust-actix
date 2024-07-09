#[cfg(test)]
mod common_tests {
    use crate::common::common::{equal_string_check, parse_query, return_type};

    #[test]
    fn test_parse_query_query() {
        let query = Some(String::from("test1=test2&test3[test1]=test1&test3[test2]=test2&test4=1"));

        assert!(equal_string_check(
            &return_type(&parse_query(query.clone())),
            &String::from("core::result::Result<std::collections::hash::map::HashMap<alloc::string::String, alloc::vec::Vec<alloc::vec::Vec<u8>>>, torrust_actix::common::structures::custom_error::CustomError>")
        ));
    }
}
