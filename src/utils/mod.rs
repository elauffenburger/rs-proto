pub enum CasedString<'a> {
    ScreamingSnakeCase(&'a str),
    SnakeCase(&'a str),
}

pub fn camel_case(string: CasedString) -> String {
    match string {
        CasedString::ScreamingSnakeCase(string) | CasedString::SnakeCase(string) => {
            let lowercased_string = string.to_lowercase();

            let mut result = vec![];
            let mut chars = lowercased_string.chars();

            while let Some(ch) = chars.next() {
                match ch {
                    '_' => {
                        if let Some(ch) = chars.next() {
                            result.push(ch.to_uppercase().next().unwrap());
                        }
                    }
                    _ => result.push(ch),
                }
            }

            result.iter().collect::<String>()
        }
    }
}
