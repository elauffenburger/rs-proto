use super::CodeGenerator;
use crate::parser::*;
use crate::utils::{camel_case, CasedString};

const BASE_ENUM_TYPE: &'static str = "ProtobufEnum";

pub struct DartCodeGenerator {
    parser: Box<Parser>,
}

impl DartCodeGenerator {
    pub fn new(parser: Box<Parser>) -> Self {
        DartCodeGenerator { parser }
    }

    fn gen_enum<'a>(
        enumeration: &ProtoEnum,
        name_prefix: &'a str,
        indent: usize,
    ) -> Result<String, String> {
        let indentation = "\t".repeat(indent as usize);
        let mut result = vec![];

        let enum_name = format!("{}{}", name_prefix, &enumeration.name);

        result.push(format!(
            "{}class {} extends {} {{\n",
            indentation, enum_name, BASE_ENUM_TYPE
        ));
        result.push(Self::gen_enum_body(&enum_name, &enumeration.values, 1)?);
        result.push(format!("\n{}}}", indentation));

        Ok(result.join(""))
    }

    fn gen_enum_body<'a>(
        enum_name: &'a str,
        enum_values: &Vec<ProtoEnumValue>,
        indent: usize,
    ) -> Result<String, String> {
        let mut result = vec![];

        for value in enum_values.iter() {
            result.push(format!(
                "{}\n",
                Self::gen_enum_value(enum_name, &value, indent)?
            ));
        }

        result.push(format!(
            "\n{}",
            Self::gen_all_enum_values_list(enum_name, enum_values, indent)?
        ));

        result.push(format!("\n\n{}", Self::gen_enum_ctor(enum_name, indent)?));

        Ok(result.join(""))
    }

    fn gen_enum_value<'a>(
        enum_name: &'a str,
        value: &ProtoEnumValue,
        indent: usize,
    ) -> Result<String, String> {
        let indentation = "\t".repeat(indent as usize);

        Ok(format!(
            "{}static {} {} = {}._(\"{}\", {});",
            indentation,
            enum_name,
            camel_case(CasedString::ScreamingSnakeCase(&value.name)),
            enum_name,
            value.name,
            value.position
        ))
    }

    fn gen_all_enum_values_list<'a>(
        enum_name: &'a str,
        enum_values: &Vec<ProtoEnumValue>,
        indent: usize,
    ) -> Result<String, String> {
        let indentation = "\t".repeat(indent as usize);
        let value_indentation = "\t".repeat(indent + 1 as usize);

        let all_values = enum_values
            .iter()
            .map(|value| {
                format!(
                    "{}{}",
                    value_indentation,
                    camel_case(CasedString::ScreamingSnakeCase(&value.name))
                )
            })
            .collect::<Vec<String>>()
            .join(",\n");

        Ok(format!(
            "{}static {}[] allValues = [\n{}\n{}];",
            indentation, enum_name, all_values, indentation
        ))
    }

    fn gen_enum_ctor<'a>(enum_name: &'a str, indent: usize) -> Result<String, String> {
        let indentation = "\t".repeat(indent as usize);
        let inner_indentation = "\t".repeat(indent + 1 as usize);

        let mut result = vec![];

        result.push(format!(
            "{}{}._(String name, int position) {{\n",
            indentation, enum_name
        ));

        result.push(format!("{}this.name = name;\n", inner_indentation));
        result.push(format!("{}this.position = position;\n", inner_indentation));

        result.push(format!("{}}}", indentation));

        Ok(result.join(""))
    }
}

impl CodeGenerator for DartCodeGenerator {
    fn gen_code<'a>(&self, src: &'a str) -> Result<String, String> {
        let mut result = vec![];

        let prog = self.parser.parse(src)?;
        for proto_type in prog.types {
            match proto_type {
                ProtoType::Enum(enumeration) => result.push(Self::gen_enum(&enumeration, "", 0)?),
                err @ _ => return Err(format!("Unknown proto type '{:?}'", err)),
            }
        }

        Ok(result.join(""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ParserImpl;

    macro_rules! gen_code_for_test {
        ($test_path: expr) => {{
            let parser = ParserImpl::new();
            let generator = DartCodeGenerator::new(Box::new(parser));

            generator
                .gen_code(include_str!($test_path))
                .expect("unsuccessful codegen")
        }};
    }

    #[test]
    fn test_enum() {
        let result = gen_code_for_test!("../../test_data/enum.proto");

        assert_eq!(
            result,
            "class RelationshipType extends ProtobufEnum {
\tstatic RelationshipType unknownValue = RelationshipType._(\"UNKNOWN_VALUE\", 0);
\tstatic RelationshipType parent = RelationshipType._(\"PARENT\", 1);
\tstatic RelationshipType sibling = RelationshipType._(\"SIBLING\", 2);
\tstatic RelationshipType child = RelationshipType._(\"CHILD\", 3);
\tstatic RelationshipType ancestor = RelationshipType._(\"ANCESTOR\", 4);
\tstatic RelationshipType descendant = RelationshipType._(\"DESCENDANT\", 5);

\tstatic RelationshipType[] allValues = [
\t\tunknownValue,
\t\tparent,
\t\tsibling,
\t\tchild,
\t\tancestor,
\t\tdescendant
\t];

\tRelationshipType._(String name, int position) {
\t\tthis.name = name;
\t\tthis.position = position;
\t}
}"
        );
    }
}
