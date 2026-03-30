use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use maud::{DOCTYPE, Markup, PreEscaped, html};

use crate::{
    constants::{APP_CSS, FONT_URL, PASTE_JS},
    response::Template,
    state::Paste,
};

pub fn index_page(error_message: Option<&str>) -> Markup {
    page(
        "Home | Rustbin",
        None,
        None,
        html! {
            form method="POST" action="/" enctype="multipart/form-data" {
                @if let Some(message) = error_message {
                    div class="notice" { (message) }
                }
                div id="prompt" { ">" }
                textarea id="code" name="content" spellcheck="false" autofocus required placeholder="Paste code or shorten a URL" {}
                (footer_home())
            }
        },
    )
}

pub fn usage_page() -> Markup {
    page(
        "Usage | Rustbin",
        None,
        None,
        html! {
            h1 { "Rustbin" }
            "A minimalist pastebin and URL shortener written in rust."
            hr;
            br;
            h2 { "Usage" }
            h3 { "From your browser" }
            p {
                "Open the homepage, paste your text, and click "
                code { "Save" }
                "."
            }
            br;
            br;
            h3 { "From your terminal" }
            pre {
                code { "$ curl -F 'file=@example.txt' https://bin.example.com/" } "\n"
            }
            pre {
                code { "$ curl -F 'file=@example.txt' -F 'expires_in=3600' https://bin.example.com/" }
            }
            p {
                "Optional field: "
                code { "expires_in" }
                " accepts "
                code { "never" }
                " or a positive number of seconds."
            }
            br;
            p {
                "You may want to use this "
                a href="https://github.com/PeroSar/rustbin-cli" { "friendly wrapper" }
                " instead."
            }
            br;
            br;
            h3 { "Viewing pastes" }
            p {
                "Open "
                code { "/{id}" }
                " to view a paste and "
                code { "/{id}/raw" }
                " for plain text."
            }
            br;
            br;
            h3 { "Syntax highlighting" }
            p {
                "Syntax highlighting is automatic based on the file extension. You can also append an extension to the paste URL:"
            }
            pre {
                code { "https://bin.example.com/abc123def4.rs" }
            }
            p { "Line links are available with fragments such as " code { "#L12" } " or " code { "#L12-L20" } "." }
            br;
            br;
            h3 { "URL shortening" }
            p {
                "Paste a URL on its own to create a short link. Visiting the short URL will redirect to the original."
            }
            pre {
                code { "$ echo 'https://example.com' | curl -F 'file=@-' https://bin.example.com/" }
            }
            (footer())
        },
    )
}

pub fn url_paste_page(short_url: &str, destination: &str) -> Markup {
    page(
        "URL | Rustbin",
        None,
        None,
        html! {
            h1 { "URL shortened" }
            hr;
            br;
            h3 { "Short URL" }
            pre {
                code {
                    a href=(short_url) { (short_url) }
                }
            }
            br;
            h3 { "Destination" }
            pre {
                code {
                    a href=(destination) { (destination) }
                }
            }
            (footer())
        },
    )
}

pub fn paste_page(paste_ref: &str, paste: &Paste, content_html: &str, is_markdown: bool) -> Markup {
    page(
        &format!("{paste_ref} | Rustbin"),
        Some(html! {
            meta name="twitter:card" content="summary_large_image";
            meta name="twitter:image" content={ "/" (paste_ref) "/preview.png" };
            meta property="og:type" content="website";
            meta property="og:image" content={ "/" (paste_ref) "/preview.png" };
        }),
        if is_markdown {
            None
        } else {
            Some(html! { script { (PreEscaped(PASTE_JS)) } })
        },
        html! {
            @if is_markdown {
                (render_markdown_block(content_html))
            } @else {
                (render_content_block(content_html))
            }
            (footer_paste(&paste.id))
        },
    )
}

fn page(
    title: &str,
    extra_head: Option<Markup>,
    extra_body_end: Option<Markup>,
    content: Markup,
) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                title { (title) }
                link rel="shortcut icon" type="image/x-icon" href="/favicon.ico";
                meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no";
                link rel="preconnect" href="https://fonts.googleapis.com";
                link rel="preconnect" href="https://fonts.gstatic.com" crossorigin;
                link rel="stylesheet" href=(FONT_URL);
                style { (PreEscaped(APP_CSS)) }
                @if let Some(extra_head) = extra_head {
                    (extra_head)
                }
            }
            body class="app-body" {
                (content)
                @if let Some(extra_body_end) = extra_body_end {
                    (extra_body_end)
                }
            }
        }
    }
}

fn render_content_block(content_html: &str) -> Markup {
    html! {
        pre class="paste-content" {
            div class="code-grid" {
                (PreEscaped(content_html))
            }
        }
    }
}

fn render_markdown_block(content_html: &str) -> Markup {
    html! {
        div class="markdown-body" {
            (PreEscaped(content_html))
        }
    }
}

pub fn render_error_response(status: StatusCode, code: &str, message: &str) -> Response {
    (status, Template(error_page(code, message))).into_response()
}

fn error_page(code: &str, message: &str) -> Markup {
    page(
        &format!("{code} | Rustbin"),
        None,
        None,
        html! {
            h1 class="title-accent" { (code) }
            p { (message) }
            br;
            p {
                a href=".." { "Homepage" }
            }
            (footer())
        },
    )
}

fn footer() -> Markup {
    footer_layout(html! {}, html! {})
}

fn footer_home() -> Markup {
    footer_layout(
        html! {},
        html! {
            span class="foot-hover" {
                button type="submit" class="link-reset foot-btn" { "Save" }
            }
            span class="foot-hover" {
                a href="/usage" class="link-reset" { "Usage" }
            }
        },
    )
}

fn footer_paste(id: &str) -> Markup {
    footer_layout(
        html! {},
        html! {
            span class="foot-hover" {
                a href={ "/" (id) "/raw" } class="link-reset" { "View raw" }
            }
            span class="foot-hover" {
                a href="/usage" class="link-reset" { "Usage" }
            }
        },
    )
}

fn footer_layout(brand_suffix: Markup, actions: Markup) -> Markup {
    html! {
        span class="foot-spacer" {}
        footer class="foot-minibuf" {
            div class="foot" {
                img src="/logo.png" height="24" class="foot-logo";
                span class="foot-hover" {
                    a href="/" class="link-reset" { "Rustbin" }
                }
                (brand_suffix)
                span class="foot-end" { (actions) }
            }
            div class="kopirite" {
                "Copyright © 2026 - Rustbin"
            }
        }
    }
}
