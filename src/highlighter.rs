use syntect::{
    easy::HighlightLines,
    highlighting::Theme,
    html::{IncludeBackground, append_highlighted_html_for_styled_line},
    parsing::{SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};
use tracing::{debug, info};

use crate::{enry_ffi, state::AppState};

pub fn render_content(state: &AppState, extension: Option<&str>, content: &str) -> String {
    match resolve_syntax(state, extension) {
        Some(syntax) => {
            render_highlighted_html(&state.syntax_set, state.theme.as_ref(), syntax, content)
        }
        None => render_plain_html(content),
    }
}

pub fn detect_language(state: &AppState, filename: Option<&str>, content: &str) -> Option<String> {
    if let Some(extension) = filename_extension(filename) {
        if resolve_syntax(state, Some(&extension)).is_some() {
            info!(
                filename = ?filename,
                extension = %extension,
                "language resolved directly from filename extension"
            );
            return Some(extension);
        }
        info!(
            filename = ?filename,
            extension = %extension,
            "filename extension did not map to a known syntax"
        );
        info!(filename = ?filename, "no stored language detected during upload");
        return None;
    }

    if let Some(classifier_extensions) = enry_ffi::detect_language_by_classifier(content) {
        for extension in classifier_extensions.split('\n') {
            let extension = extension.trim();
            if extension.is_empty() {
                continue;
            }
            if resolve_syntax(state, Some(extension)).is_some() {
                debug!(
                    filename = ?filename,
                    extension = %extension,
                    "language resolved from enry classifier"
                );
                return Some(extension.to_string());
            }
        }

        info!(
            filename = ?filename,
            classifier_extensions = %classifier_extensions,
            "enry classifier extensions did not map to a known syntax"
        );
    }

    info!(filename = ?filename, "no stored language detected during upload");
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
    use syntect::parsing::SyntaxSet;

    use super::{filename_extension, stored_language_for_syntax};

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
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let syntax = syntax_set
            .find_syntax_by_name("Rust")
            .expect("Rust syntax must exist");

        assert_eq!(stored_language_for_syntax(syntax), "rs");
    }
}
