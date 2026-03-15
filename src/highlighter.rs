use syntect::{
    easy::HighlightLines,
    highlighting::Theme,
    html::{IncludeBackground, append_highlighted_html_for_styled_line},
    parsing::{SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::{enry_ffi, state::AppState};

pub fn render_content(state: &AppState, extension: Option<&str>, content: &str) -> String {
    match resolve_syntax(state, extension) {
        Some(syntax) => {
            render_highlighted_html(&state.syntax_set, state.theme.as_ref(), syntax, content)
        }
        None => render_plain_html(content),
    }
}

pub fn is_markdown(extension: Option<&str>) -> bool {
    extension.is_some_and(|e| {
        let e = e.trim();
        e.eq_ignore_ascii_case("md")
            || e.eq_ignore_ascii_case("markdown")
            || e.eq_ignore_ascii_case("mdown")
            || e.eq_ignore_ascii_case("mkd")
            || e.eq_ignore_ascii_case("mkdn")
    })
}

pub fn render_markdown(state: &AppState, content: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_SMART_PUNCTUATION;

    let parser = Parser::new_ext(content, options);

    let mut events: Vec<Event<'_>> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang: Option<String> = None;
    let mut code_text = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_lang = match &kind {
                    CodeBlockKind::Fenced(lang) => {
                        let l = lang.split_whitespace().next().unwrap_or("");
                        if l.is_empty() {
                            None
                        } else {
                            Some(l.to_string())
                        }
                    }
                    CodeBlockKind::Indented => None,
                };
                code_text.clear();
            }
            Event::Text(text) if in_code_block => {
                code_text.push_str(&text);
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                let html = render_markdown_code_block(state, code_lang.as_deref(), &code_text);
                events.push(Event::Html(html.into()));
                code_lang = None;
            }
            _ => events.push(event),
        }
    }

    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, events.into_iter());
    html_output
}

fn render_markdown_code_block(state: &AppState, lang: Option<&str>, code: &str) -> String {
    let syntax = lang.and_then(|l| resolve_syntax(state, Some(l)));

    let mut html = String::new();
    match lang {
        Some(l) => {
            html.push_str("<pre><code class=\"language-");
            push_escaped_html(&mut html, l);
            html.push_str("\">");
        }
        None => html.push_str("<pre><code>"),
    }

    if let Some(syntax) = syntax {
        let mut highlighter = HighlightLines::new(syntax, state.theme.as_ref());
        for line in LinesWithEndings::from(code) {
            match highlighter.highlight_line(line, &state.syntax_set) {
                Ok(regions) => {
                    let mut line_html = String::new();
                    if append_highlighted_html_for_styled_line(
                        &regions,
                        IncludeBackground::No,
                        &mut line_html,
                    )
                    .is_err()
                    {
                        push_escaped_html(&mut html, line);
                    } else {
                        html.push_str(&line_html);
                    }
                }
                Err(_) => push_escaped_html(&mut html, line),
            }
        }
    } else {
        push_escaped_html(&mut html, code);
    }

    html.push_str("</code></pre>\n");
    html
}

pub fn detect_language(state: &AppState, filename: Option<&str>, content: &str) -> Option<String> {
    if let Some(extension) = filename_extension(filename) {
        if resolve_syntax(state, Some(&extension)).is_some() {
            return Some(extension);
        }
        return None;
    }

    if let Some(classifier_extensions) = enry_ffi::detect_language_by_classifier(content) {
        for extension in classifier_extensions.split('\n') {
            let extension = extension.trim();
            if extension.is_empty() {
                continue;
            }
            if resolve_syntax(state, Some(extension)).is_some() {
                return Some(extension.to_string());
            }
        }
    }

    None
}

fn filename_extension(filename: Option<&str>) -> Option<String> {
    let (_, extension) = filename?.rsplit_once('.')?;
    let extension = extension.trim().trim_start_matches('.');
    if extension.is_empty() {
        return None;
    }

    Some(extension.to_ascii_lowercase())
}

fn resolve_syntax<'a>(state: &'a AppState, extension: Option<&str>) -> Option<&'a SyntaxReference> {
    let extension = extension?
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    if extension.is_empty() {
        return None;
    }

    let &index = state.syntax_index_by_token.get(&extension)?;
    state.syntax_set.syntaxes().get(index)
}

#[cfg(test)]
fn stored_language_for_syntax(syntax: &SyntaxReference) -> String {
    syntax
        .file_extensions
        .first()
        .cloned()
        .unwrap_or_else(|| syntax.name.clone())
}

fn render_highlighted_html(
    syntax_set: &SyntaxSet,
    theme: &Theme,
    syntax: &SyntaxReference,
    content: &str,
) -> String {
    if content.is_empty() {
        return render_line_html(1, String::new());
    }

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut rendered = String::with_capacity(estimated_rendered_capacity(content));
    let mut line_number = 1usize;
    let mut line_html = String::new();

    for line in LinesWithEndings::from(content) {
        line_html.clear();

        match highlighter.highlight_line(line, syntax_set) {
            Ok(regions) => {
                if append_highlighted_html_for_styled_line(
                    &regions,
                    IncludeBackground::No,
                    &mut line_html,
                )
                .is_err()
                {
                    line_html.clear();
                    push_escaped_html(&mut line_html, trim_line_ending(line));
                }
            }
            Err(_) => push_escaped_html(&mut line_html, trim_line_ending(line)),
        }

        push_line_html(&mut rendered, line_number, &line_html);
        line_number += 1;
    }

    rendered
}

fn render_plain_html(content: &str) -> String {
    if content.is_empty() {
        return render_line_html(1, String::new());
    }

    let mut rendered = String::with_capacity(estimated_rendered_capacity(content));
    let mut line_number = 1usize;
    let mut line_html = String::new();

    for line in LinesWithEndings::from(content) {
        line_html.clear();
        push_escaped_html(&mut line_html, trim_line_ending(line));
        push_line_html(&mut rendered, line_number, &line_html);
        line_number += 1;
    }

    rendered
}

fn estimated_rendered_capacity(content: &str) -> usize {
    let line_count = bytecount::count(content.as_bytes(), b'\n').saturating_add(1);
    content.len().saturating_mul(3) + line_count * 200
}

fn render_line_html(line_number: usize, line_html: String) -> String {
    let mut rendered = String::new();
    push_line_html(&mut rendered, line_number, &line_html);
    rendered
}

fn push_line_html(output: &mut String, line_number: usize, line_html: &str) {
    let mut buf = itoa::Buffer::new();
    let n = buf.format(line_number);

    output.push_str("<code id=\"L");
    output.push_str(n);
    output.push_str("\" class=\"code-line\" data-line-number=\"");
    output.push_str(n);
    output.push_str("\"><a class=\"line-link\" href=\"#L");
    output.push_str(n);
    output.push_str("\" data-line-number=\"");
    output.push_str(n);
    output.push_str("\" aria-label=\"Link to line ");
    output.push_str(n);
    output.push_str("\">");
    output.push_str(n);
    output.push_str("</a><span class=\"line-content\">");
    output.push_str(line_html);
    output.push_str("</span></code>");
}

fn push_escaped_html(output: &mut String, value: &str) {
    let bytes = value.as_bytes();
    let mut last = 0;
    for (i, &b) in bytes.iter().enumerate() {
        let replacement = match b {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'"' => "&quot;",
            b'\'' => "&#39;",
            _ => continue,
        };
        output.push_str(&value[last..i]);
        output.push_str(replacement);
        last = i + 1;
    }
    output.push_str(&value[last..]);
}

fn trim_line_ending(line: &str) -> &str {
    line.strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .or_else(|| line.strip_suffix('\r'))
        .unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use syntect::dumps::from_uncompressed_data;
    use syntect::parsing::SyntaxSet;

    use super::{filename_extension, stored_language_for_syntax};

    fn load_syntax_set() -> SyntaxSet {
        from_uncompressed_data(include_bytes!("../syntaxes.bin"))
            .expect("failed to load syntaxes.bin")
    }

    #[test]
    fn extracts_lowercased_extension() {
        assert_eq!(
            filename_extension(Some("src/main.RS")),
            Some("rs".to_string())
        );
    }

    #[test]
    fn ignores_missing_extension() {
        assert_eq!(filename_extension(Some("Dockerfile")), None);
        assert_eq!(filename_extension(Some("file.")), None);
        assert_eq!(filename_extension(None), None);
    }

    #[test]
    fn prefers_extension_for_stored_language() {
        let syntax_set = load_syntax_set();
        let syntax = syntax_set
            .find_syntax_by_name("Rust")
            .expect("Rust syntax must exist");

        assert_eq!(stored_language_for_syntax(syntax), "rs");
    }
}
