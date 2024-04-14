use ropey::Rope;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Highlight the text stored in the given rope and return a list of highlighted lines.
pub fn highlight(rope: &Rope, extension: &str) -> Vec<Vec<(Style, String)>> {
    // setup
    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();

    // create syntax based on extension, select theme and extract string from rope
    let syntax = syntax_set.find_syntax_by_extension(extension).unwrap();
    let mut highlight_lines = HighlightLines::new(syntax, &theme_set.themes["base16-ocean.dark"]);
    let string = rope.to_string();

    // keep track of highlighted lines in a results vector
    let mut result: Vec<Vec<(Style, String)>> = Vec::new();

    for line in LinesWithEndings::from(&string) {
        // map the highlighted strings from a referenced str to an owned one
        let highlighted_line = Vec::from_iter(
            highlight_lines
                .highlight_line(line, &syntax_set)
                .unwrap()
                .iter()
                .map(|(style, string)| (*style, String::from(*string))),
        );
        result.push(highlighted_line);
    }
    result
}

pub fn convert_color(from: syntect::highlighting::Color) -> notan::prelude::Color {
    notan::prelude::Color::from_bytes(from.r, from.g, from.b, from.a)
}
