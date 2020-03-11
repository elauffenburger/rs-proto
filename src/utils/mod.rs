pub enum CasedString<'a> {
    ScreamingSnakeCase(&'a str),
}

pub fn camel_case<'a>(string: CasedString) -> String {
    match string {
        CasedString::ScreamingSnakeCase(string) => {
            let lowercased_string = string.to_lowercase();

            let mut result = vec![];
            let mut chars = lowercased_string.chars();

            while let Some(ch) = chars.next() {
                match ch {
                    '_' => match chars.next() {
                        Some(ch) => {
                            result.push(ch.to_uppercase().next().unwrap());
                        }
                        _ => {}
                    },
                    _ => result.push(ch),
                }
            }

            result.iter().collect::<String>()
        }
    }
}

pub fn pascal_case<'a>(string: &'a str) -> String {
    let mut result = string.to_lowercase().chars().collect::<Vec<char>>();
    result[0] = result[0].to_uppercase().nth(0).unwrap();

    result.iter().collect()
}
