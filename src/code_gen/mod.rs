mod dart;
mod env;

use dart::DartCodeGenerator;
use crate::parser::Parser;

pub enum Language {
    Dart,
}

pub trait CodeGenerator {
    fn gen_code<'a>(&self, src: &'a str) -> Result<String, String>;
}

pub fn generator_for(parser: Box<Parser>, language: Language) -> impl CodeGenerator {
    match language {
        Language::Dart => DartCodeGenerator::new(parser),
    }
}
