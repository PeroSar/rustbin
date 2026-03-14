use std::path::Path;

use syntect::dumps::dump_to_uncompressed_file;
use syntect::parsing::SyntaxSetBuilder;

fn main() {
    let syntax_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "bat/assets/syntaxes".to_string());

    eprintln!("loading syntaxes from {syntax_dir}...");

    let mut builder = SyntaxSetBuilder::new();
    builder
        .add_from_folder(Path::new(&syntax_dir), true)
        .unwrap_or_else(|error| panic!("failed to load syntaxes from {syntax_dir}: {error}"));

    let syntax_set = builder.build();
    eprintln!("loaded {} syntaxes", syntax_set.syntaxes().len());

    let output = "syntaxes.bin";
    dump_to_uncompressed_file(&syntax_set, output)
        .unwrap_or_else(|error| panic!("failed to write {output}: {error}"));

    eprintln!("written to {output}");
}
