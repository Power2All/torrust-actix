#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use crate::common::parse_query;

    #[test]
    fn test_parse_query() {
        let result = parse_query(Some(String::from("test1=test2&test1=test3&test7=test8&test5&test6")));
        assert_eq!(Vec::from_iter(result.unwrap().iter().sorted()), vec![
            (&String::from("test1"), &vec![
                String::from("test2").as_bytes().to_vec(),
                String::from("test3").as_bytes().to_vec()
            ]),
            (&String::from("test5"), &vec![]),
            (&String::from("test6"), &vec![]),
            (&String::from("test7"), &vec![
                String::from("test8").as_bytes().to_vec()
            ])
        ]);
    }
}