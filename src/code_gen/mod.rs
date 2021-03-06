mod dart;
mod env;

use crate::parser::Parser;
use dart::DartCodeGenerator;

pub enum Language {
    Dart,
}

pub trait CodeGenerator {
    fn gen_code(&self, src: String) -> Result<String, String>;
}

pub fn generator_for(parser: Box<Parser>, language: Language) -> impl CodeGenerator {
    match language {
        Language::Dart => DartCodeGenerator::new(parser),
    }
}
