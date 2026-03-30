use std::collections::{HashMap, hash_map::Entry};

use fontdue::{Font, FontSettings};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use syntect::{easy::HighlightLines, highlighting::Style, util::LinesWithEndings};

use crate::{
    highlighter::{is_markdown, resolve_syntax},
    state::AppState,
};

const WIDTH: usize = 1200;
const HEIGHT: usize = 630;
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: usize = 22;
const PADDING_X: usize = 16;
const PADDING_Y: usize = 14;
const MAX_LINES: usize = 25;
const LINE_NUMBER_WIDTH: usize = 44;
const TAB_WIDTH: usize = 4;

const BG_R: u8 = 0x0a;
const BG_G: u8 = 0x0c;
const BG_B: u8 = 0x10;

const MUTED_R: u8 = 0x9e;
const MUTED_G: u8 = 0xa7;
const MUTED_B: u8 = 0xb3;

const FG_R: u8 = 0xf0;
const FG_G: u8 = 0xf3;
const FG_B: u8 = 0xf6;

// Accent color (--accent: #71b7ff) for links
const ACCENT_R: u8 = 0x71;
const ACCENT_G: u8 = 0xb7;
const ACCENT_B: u8 = 0xff;

// Border color (--border: #272b33)
const BORDER_R: u8 = 0x27;
const BORDER_G: u8 = 0x2b;
const BORDER_B: u8 = 0x33;

// Code block background (--panel-2: #0f141b)
const PANEL2_R: u8 = 0x0f;
const PANEL2_G: u8 = 0x14;
const PANEL2_B: u8 = 0x1b;

// Inline code background (--panel: #161b22)
const PANEL_R: u8 = 0x16;
const PANEL_G: u8 = 0x1b;
const PANEL_B: u8 = 0x22;

// Markdown padding (matches CSS: padding: 32px 40px)
const MD_PADDING_X: usize = 40;
const MD_PADDING_Y: usize = 32;

// Markdown font sizes (matching CSS em values relative to 14px base)
const MD_H1_SIZE: f32 = 28.0; // 2em
const MD_H2_SIZE: f32 = 21.0; // 1.5em
const MD_H3_SIZE: f32 = 17.5; // 1.25em
const MD_CODE_SIZE: f32 = 12.0; // .85em

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct GlyphCacheKey {
    ch: char,
    font_size_bits: u32,
}

struct CachedGlyph {
    width: usize,
    height: usize,
    xmin: i32,
    ymin: i32,
    advance_width: usize,
    bitmap: Box<[u8]>,
}

#[derive(Default)]
struct GlyphCache {
    glyphs: HashMap<GlyphCacheKey, CachedGlyph>,
}

pub fn load_font() -> Font {
    let font_data = include_bytes!("../font/DMMono-Regular.ttf");
    Font::from_bytes(font_data as &[u8], FontSettings::default())
        .expect("failed to load embedded font")
}

pub fn generate_preview(state: &AppState, content: &str, extension: Option<&str>) -> Vec<u8> {
    let mut pixels = [BG_R, BG_G, BG_B, 255u8].repeat(WIDTH * HEIGHT);
    let mut glyph_cache = GlyphCache::default();

    if content.is_empty() {
        return encode_png(&pixels);
    }

    if is_markdown(extension) {
        return generate_markdown_preview(state, &mut pixels, content, &mut glyph_cache);
    }

    let syntax = resolve_syntax(state, extension);
    let lines: Vec<&str> = LinesWithEndings::from(content)
        .take(MAX_LINES + 1)
        .collect();
    let has_more = lines.len() > MAX_LINES;
    let visible_lines = if has_more { MAX_LINES } else { lines.len() };

    let font = &state.font;

    // Render each line
    let mut highlighter = syntax.map(|s| HighlightLines::new(s, state.theme.as_ref()));

    let mut num_buf = itoa::Buffer::new();

    for (line_idx, &line) in lines.iter().take(visible_lines).enumerate() {
        let y_offset = PADDING_Y + line_idx * LINE_HEIGHT;
        if y_offset + LINE_HEIGHT > HEIGHT {
            break;
        }

        let line_num = line_idx + 1;

        // Render line number (right-aligned in LINE_NUMBER_WIDTH area)
        let num_str = num_buf.format(line_num);
        render_text_right_aligned(
            &mut pixels,
            font,
            &mut glyph_cache,
            num_str,
            PADDING_X + LINE_NUMBER_WIDTH - 8,
            y_offset,
            MUTED_R,
            MUTED_G,
            MUTED_B,
        );

        // Determine fade factor for last 3 lines if there's more content
        let alpha_factor = if has_more && line_idx >= visible_lines - 3 {
            let fade_pos = visible_lines - line_idx;
            match fade_pos {
                3 => 200u8,
                2 => 140u8,
                _ => 80u8,
            }
        } else {
            255u8
        };

        // Syntax-highlighted content
        let trimmed = trim_line_ending(line);
        let content_x = PADDING_X + LINE_NUMBER_WIDTH + 8;

        if let Some(ref mut hl) = highlighter {
            match hl.highlight_line(line, &state.syntax_set) {
                Ok(regions) => {
                    render_highlighted_regions(
                        &mut pixels,
                        font,
                        &mut glyph_cache,
                        &regions,
                        content_x,
                        y_offset,
                        alpha_factor,
                    );
                }
                Err(_) => {
                    render_text(
                        &mut pixels,
                        font,
                        &mut glyph_cache,
                        trimmed,
                        content_x,
                        y_offset,
                        apply_alpha(FG_R, alpha_factor),
                        apply_alpha(FG_G, alpha_factor),
                        apply_alpha(FG_B, alpha_factor),
                    );
                }
            }
        } else {
            render_text(
                &mut pixels,
                font,
                &mut glyph_cache,
                trimmed,
                content_x,
                y_offset,
                apply_alpha(FG_R, alpha_factor),
                apply_alpha(FG_G, alpha_factor),
                apply_alpha(FG_B, alpha_factor),
            );
        }
    }

    // If truncated, render "..." on the next line
    if has_more {
        let dots_y = PADDING_Y + visible_lines * LINE_HEIGHT;
        if dots_y + LINE_HEIGHT <= HEIGHT {
            let content_x = PADDING_X + LINE_NUMBER_WIDTH + 8;
            render_text(
                &mut pixels,
                font,
                &mut glyph_cache,
                "...",
                content_x,
                dots_y,
                MUTED_R,
                MUTED_G,
                MUTED_B,
            );
        }
    }

    encode_png(&pixels)
}

// --- Markdown preview rendering ---

#[derive(Clone)]
struct MdSpan {
    text: String,
    r: u8,
    g: u8,
    b: u8,
    font_size: f32,
    has_bg: bool, // inline code background
}

impl MdSpan {
    fn new(text: impl Into<String>, r: u8, g: u8, b: u8) -> Self {
        Self {
            text: text.into(),
            r,
            g,
            b,
            font_size: FONT_SIZE,
            has_bg: false,
        }
    }

    fn sized(text: impl Into<String>, r: u8, g: u8, b: u8, font_size: f32) -> Self {
        Self {
            text: text.into(),
            r,
            g,
            b,
            font_size,
            has_bg: false,
        }
    }

    fn code(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            r: FG_R,
            g: FG_G,
            b: FG_B,
            font_size: MD_CODE_SIZE,
            has_bg: true,
        }
    }
}

struct MdLine {
    spans: Vec<MdSpan>,
    line_height: usize,
    has_underline: bool,     // h1/h2 border-bottom
    is_code_block: bool,     // code block background
    blockquote_depth: usize, // left border count
    is_rule: bool,
}

impl MdLine {
    fn empty() -> Self {
        Self {
            spans: Vec::new(),
            line_height: LINE_HEIGHT,
            has_underline: false,
            is_code_block: false,
            blockquote_depth: 0,
            is_rule: false,
        }
    }

    fn text(spans: Vec<MdSpan>) -> Self {
        Self {
            spans,
            ..Self::empty()
        }
    }
}

/// Collect styled lines from parsed markdown, then render them into the pixel buffer.
fn generate_markdown_preview(
    state: &AppState,
    pixels: &mut [u8],
    content: &str,
    glyph_cache: &mut GlyphCache,
) -> Vec<u8> {
    let lines = markdown_to_styled_lines(state, content);

    let font = &state.font;
    let mut y = MD_PADDING_Y;
    let visible_count = lines.len();

    // Find how many lines fit
    let mut last_visible = 0;
    {
        let mut check_y = MD_PADDING_Y;
        for (i, line) in lines.iter().enumerate() {
            if check_y + line.line_height > HEIGHT - MD_PADDING_Y {
                break;
            }
            check_y += line.line_height;
            last_visible = i + 1;
        }
    }
    let has_more = last_visible < visible_count;

    for (line_idx, line) in lines.iter().take(last_visible).enumerate() {
        if y + line.line_height > HEIGHT {
            break;
        }

        let alpha_factor = if has_more && line_idx >= last_visible.saturating_sub(3) {
            match last_visible - line_idx {
                3 => 200u8,
                2 => 140u8,
                _ => 80u8,
            }
        } else {
            255u8
        };

        let content_x = MD_PADDING_X + line.blockquote_depth * 16; // blockquote indent

        // Draw blockquote left border(s)
        if line.blockquote_depth > 0 {
            for depth in 0..line.blockquote_depth {
                let border_x = MD_PADDING_X + depth * 16;
                let br = apply_alpha(BORDER_R, alpha_factor);
                let bg = apply_alpha_channel(BORDER_G, BG_G, alpha_factor);
                let bb = apply_alpha_channel(BORDER_B, BG_B, alpha_factor);
                fill_rect(pixels, border_x, y, 3, line.line_height, br, bg, bb);
            }
        }

        // Draw code block background
        if line.is_code_block {
            let bg_r = apply_alpha_channel(PANEL2_R, BG_R, alpha_factor);
            let bg_g = apply_alpha_channel(PANEL2_G, BG_G, alpha_factor);
            let bg_b = apply_alpha_channel(PANEL2_B, BG_B, alpha_factor);
            fill_rect(
                pixels,
                content_x,
                y,
                WIDTH - content_x - MD_PADDING_X,
                line.line_height,
                bg_r,
                bg_g,
                bg_b,
            );
        }

        // Draw horizontal rule
        if line.is_rule {
            let hr_y = y + line.line_height / 2;
            let hr_r = apply_alpha(BORDER_R, alpha_factor);
            let hr_g = apply_alpha_channel(BORDER_G, BG_G, alpha_factor);
            let hr_b = apply_alpha_channel(BORDER_B, BG_B, alpha_factor);
            fill_rect(
                pixels,
                content_x,
                hr_y,
                WIDTH - content_x - MD_PADDING_X,
                3,
                hr_r,
                hr_g,
                hr_b,
            );
        } else {
            // Render text spans
            let text_x = if line.is_code_block {
                content_x + 16
            } else {
                content_x
            };
            let mut cursor_x = text_x;
            for span in &line.spans {
                let r = apply_alpha(span.r, alpha_factor);
                let g = apply_alpha_channel(span.g, BG_G, alpha_factor);
                let b = apply_alpha_channel(span.b, BG_B, alpha_factor);

                // Draw inline code background
                if span.has_bg {
                    let text_w = measure_text_width(glyph_cache, font, &span.text, span.font_size);
                    let pad = 3;
                    let bg_r = apply_alpha_channel(PANEL_R, BG_R, alpha_factor);
                    let bg_g = apply_alpha_channel(PANEL_G, BG_G, alpha_factor);
                    let bg_b = apply_alpha_channel(PANEL_B, BG_B, alpha_factor);
                    let bg_y = y + 2;
                    let bg_h = line.line_height.saturating_sub(4);
                    fill_rect(
                        pixels,
                        cursor_x.saturating_sub(pad),
                        bg_y,
                        text_w + pad * 2,
                        bg_h,
                        bg_r,
                        bg_g,
                        bg_b,
                    );
                }

                cursor_x = render_text_sized(
                    pixels,
                    font,
                    glyph_cache,
                    &span.text,
                    cursor_x,
                    y,
                    r,
                    g,
                    b,
                    span.font_size,
                    line.line_height,
                    WIDTH - MD_PADDING_X,
                );
            }
        }

        // Draw h1/h2 underline
        if line.has_underline {
            let ul_y = y + line.line_height - 2;
            let ul_r = apply_alpha(BORDER_R, alpha_factor);
            let ul_g = apply_alpha_channel(BORDER_G, BG_G, alpha_factor);
            let ul_b = apply_alpha_channel(BORDER_B, BG_B, alpha_factor);
            fill_rect(
                pixels,
                content_x,
                ul_y,
                WIDTH - content_x - MD_PADDING_X,
                1,
                ul_r,
                ul_g,
                ul_b,
            );
        }

        y += line.line_height;
    }

    if has_more && y + LINE_HEIGHT <= HEIGHT {
        render_text(
            pixels,
            font,
            glyph_cache,
            "...",
            MD_PADDING_X,
            y,
            MUTED_R,
            MUTED_G,
            MUTED_B,
        );
    }

    encode_png(pixels)
}

/// Parse markdown with pulldown-cmark and produce styled lines for the preview image.
fn markdown_to_styled_lines(state: &AppState, content: &str) -> Vec<MdLine> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_SMART_PUNCTUATION;

    let parser = Parser::new_ext(content, options);

    let mut lines: Vec<MdLine> = Vec::new();
    let mut current_spans: Vec<MdSpan> = Vec::new();

    let mut heading_level: Option<u8> = None;
    let mut in_block_quote = false;
    let mut blockquote_depth: usize = 0;
    let mut in_code_block = false;
    let mut code_block_lang: Option<String> = None;
    let mut code_block_text = String::new();
    let mut in_link = false;
    let mut list_indent: usize = 0;
    let mut ordered_index: Option<u64> = None;
    let mut need_block_gap = false;

    let max_lines = 60; // generous limit; we trim by pixel height later

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                if need_block_gap {
                    lines.push(MdLine::empty());
                }
                heading_level = Some(level as u8);
                need_block_gap = false;
                current_spans.clear();
            }
            Event::End(TagEnd::Heading(level)) => {
                let lvl = level as u8;
                let (font_size, line_height) = match lvl {
                    1 => (MD_H1_SIZE, 44),
                    2 => (MD_H2_SIZE, 36),
                    3 => (MD_H3_SIZE, 30),
                    _ => (FONT_SIZE, LINE_HEIGHT),
                };
                // Rewrite span font sizes
                for span in &mut current_spans {
                    span.font_size = font_size;
                }
                let mut line = MdLine {
                    spans: std::mem::take(&mut current_spans),
                    line_height,
                    has_underline: lvl <= 2,
                    is_code_block: false,
                    blockquote_depth,
                    is_rule: false,
                };
                if lvl == 6 {
                    for span in &mut line.spans {
                        span.r = MUTED_R;
                        span.g = MUTED_G;
                        span.b = MUTED_B;
                    }
                }
                lines.push(line);
                heading_level = None;
                need_block_gap = true;
            }
            Event::Start(Tag::Paragraph) => {
                if need_block_gap {
                    lines.push(MdLine::empty());
                }
                need_block_gap = false;
                current_spans.clear();
            }
            Event::End(TagEnd::Paragraph) => {
                if !current_spans.is_empty() {
                    let mut line = MdLine::text(std::mem::take(&mut current_spans));
                    line.blockquote_depth = blockquote_depth;
                    lines.push(line);
                }
                need_block_gap = true;
            }
            Event::Start(Tag::BlockQuote(_)) => {
                if need_block_gap && blockquote_depth == 0 {
                    lines.push(MdLine::empty());
                }
                blockquote_depth += 1;
                in_block_quote = blockquote_depth > 0;
                need_block_gap = false;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                blockquote_depth = blockquote_depth.saturating_sub(1);
                in_block_quote = blockquote_depth > 0;
                need_block_gap = true;
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                if need_block_gap {
                    lines.push(MdLine::empty());
                }
                in_code_block = true;
                code_block_lang = match &kind {
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
                code_block_text.clear();
                need_block_gap = false;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                render_code_block_to_md_lines(
                    state,
                    &code_block_text,
                    code_block_lang.as_deref(),
                    blockquote_depth,
                    &mut lines,
                );
                code_block_lang = None;
                need_block_gap = true;
            }
            Event::Start(Tag::List(first_item)) => {
                if need_block_gap && list_indent == 0 {
                    lines.push(MdLine::empty());
                }
                ordered_index = first_item;
                list_indent += 1;
                need_block_gap = false;
            }
            Event::End(TagEnd::List(_)) => {
                list_indent = list_indent.saturating_sub(1);
                ordered_index = None;
                if list_indent == 0 {
                    need_block_gap = true;
                }
            }
            Event::Start(Tag::Item) => {
                current_spans.clear();
                let indent = "  ".repeat(list_indent.saturating_sub(1));
                let bullet = if let Some(idx) = &mut ordered_index {
                    let s = format!("{indent}{}. ", idx);
                    *idx += 1;
                    s
                } else {
                    format!("{indent}• ")
                };
                current_spans.push(MdSpan::new(bullet, MUTED_R, MUTED_G, MUTED_B));
            }
            Event::End(TagEnd::Item) => {
                if !current_spans.is_empty() {
                    let mut line = MdLine::text(std::mem::take(&mut current_spans));
                    line.blockquote_depth = blockquote_depth;
                    lines.push(line);
                }
            }
            Event::Start(Tag::Link { .. }) => {
                in_link = true;
            }
            Event::End(TagEnd::Link) => {
                in_link = false;
            }
            Event::Start(Tag::Emphasis | Tag::Strong | Tag::Strikethrough) => {}
            Event::End(TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough) => {}
            Event::Code(text) => {
                current_spans.push(MdSpan::code(text.to_string()));
            }
            Event::Text(text) if in_code_block => {
                code_block_text.push_str(&text);
            }
            Event::Text(text) => {
                let (r, g, b) = if in_link {
                    (ACCENT_R, ACCENT_G, ACCENT_B)
                } else if in_block_quote && heading_level.is_none() {
                    (MUTED_R, MUTED_G, MUTED_B)
                } else {
                    (FG_R, FG_G, FG_B)
                };

                for (i, part) in text.split('\n').enumerate() {
                    if i > 0 {
                        let mut line = MdLine::text(std::mem::take(&mut current_spans));
                        line.blockquote_depth = blockquote_depth;
                        lines.push(line);
                    }
                    if !part.is_empty() {
                        current_spans.push(MdSpan::new(part.to_string(), r, g, b));
                    }
                }
            }
            Event::SoftBreak => {
                current_spans.push(MdSpan::new(" ", FG_R, FG_G, FG_B));
            }
            Event::HardBreak => {
                let mut line = MdLine::text(std::mem::take(&mut current_spans));
                line.blockquote_depth = blockquote_depth;
                lines.push(line);
            }
            Event::Rule => {
                if need_block_gap {
                    lines.push(MdLine::empty());
                }
                let mut line = MdLine::empty();
                line.is_rule = true;
                line.line_height = 24;
                line.blockquote_depth = blockquote_depth;
                lines.push(line);
                need_block_gap = true;
            }
            _ => {}
        }

        if lines.len() > max_lines {
            break;
        }
    }

    if !current_spans.is_empty() {
        let mut line = MdLine::text(current_spans);
        line.blockquote_depth = blockquote_depth;
        lines.push(line);
    }

    lines
}

/// Render a code block's lines (with optional syntax highlighting) into markdown styled lines.
fn render_code_block_to_md_lines(
    state: &AppState,
    code: &str,
    lang: Option<&str>,
    blockquote_depth: usize,
    lines: &mut Vec<MdLine>,
) {
    let syntax = lang.and_then(|l| resolve_syntax(state, Some(l)));

    if let Some(syntax) = syntax {
        let mut hl = HighlightLines::new(syntax, state.theme.as_ref());
        for line_text in LinesWithEndings::from(code) {
            let mut spans: Vec<MdSpan> = Vec::new();
            match hl.highlight_line(line_text, &state.syntax_set) {
                Ok(regions) => {
                    for (style, text) in &regions {
                        let text = trim_line_ending(text);
                        if !text.is_empty() {
                            spans.push(MdSpan::sized(
                                text.to_string(),
                                style.foreground.r,
                                style.foreground.g,
                                style.foreground.b,
                                MD_CODE_SIZE,
                            ));
                        }
                    }
                }
                Err(_) => {
                    spans.push(MdSpan::sized(
                        trim_line_ending(line_text).to_string(),
                        FG_R,
                        FG_G,
                        FG_B,
                        MD_CODE_SIZE,
                    ));
                }
            }
            lines.push(MdLine {
                spans,
                line_height: 20,
                has_underline: false,
                is_code_block: true,
                blockquote_depth,
                is_rule: false,
            });
        }
    } else {
        for line_text in LinesWithEndings::from(code) {
            lines.push(MdLine {
                spans: vec![MdSpan::sized(
                    trim_line_ending(line_text).to_string(),
                    FG_R,
                    FG_G,
                    FG_B,
                    MD_CODE_SIZE,
                )],
                line_height: 20,
                has_underline: false,
                is_code_block: true,
                blockquote_depth,
                is_rule: false,
            });
        }
    }
}

fn fill_rect(pixels: &mut [u8], x: usize, y: usize, w: usize, h: usize, r: u8, g: u8, b: u8) {
    if x >= WIDTH || y >= HEIGHT || w == 0 || h == 0 {
        return;
    }
    let x_end = (x + w).min(WIDTH);
    let y_end = (y + h).min(HEIGHT);
    for row in y..y_end {
        let row_offset = row * WIDTH * 4;
        for pixel in pixels[row_offset + x * 4..row_offset + x_end * 4].chunks_exact_mut(4) {
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
        }
    }
}

fn measure_text_width(
    glyph_cache: &mut GlyphCache,
    font: &Font,
    text: &str,
    font_size: f32,
) -> usize {
    let space_advance = glyph_advance(glyph_cache, font, ' ', font_size);
    let mut width = 0;

    for ch in text.chars() {
        width += if ch == '\t' {
            space_advance * TAB_WIDTH
        } else {
            glyph_advance(glyph_cache, font, ch, font_size)
        };
    }

    width
}

fn render_text_sized(
    pixels: &mut [u8],
    font: &Font,
    glyph_cache: &mut GlyphCache,
    text: &str,
    start_x: usize,
    y_offset: usize,
    r: u8,
    g: u8,
    b: u8,
    font_size: f32,
    line_height: usize,
    max_x: usize,
) -> usize {
    let mut cursor_x = start_x;
    let space_advance = glyph_advance(glyph_cache, font, ' ', font_size);

    for ch in text.chars() {
        if ch == '\t' {
            cursor_x += space_advance * TAB_WIDTH;
            continue;
        }

        if cursor_x >= max_x {
            break;
        }

        let glyph = cached_glyph(glyph_cache, font, ch, font_size);
        if glyph.width == 0 || glyph.height == 0 {
            cursor_x += glyph.advance_width;
            continue;
        }

        draw_glyph(pixels, glyph, cursor_x, y_offset, line_height, r, g, b);
        cursor_x += glyph.advance_width;
    }

    cursor_x
}

fn apply_alpha(color: u8, alpha: u8) -> u8 {
    if alpha == 255 {
        return color;
    }
    // Blend with background
    let bg = BG_R; // approximate - all bg channels are similar
    ((color as u16 * alpha as u16 + bg as u16 * (255 - alpha as u16)) / 255) as u8
}

fn apply_alpha_channel(fg: u8, bg: u8, alpha: u8) -> u8 {
    if alpha == 255 {
        return fg;
    }
    ((fg as u16 * alpha as u16 + bg as u16 * (255 - alpha as u16)) / 255) as u8
}

fn render_highlighted_regions(
    pixels: &mut [u8],
    font: &Font,
    glyph_cache: &mut GlyphCache,
    regions: &[(Style, &str)],
    start_x: usize,
    y_offset: usize,
    alpha_factor: u8,
) {
    let mut cursor_x = start_x;

    for &(style, text) in regions {
        let text = trim_line_ending(text);
        let r = apply_alpha_channel(style.foreground.r, BG_R, alpha_factor);
        let g = apply_alpha_channel(style.foreground.g, BG_G, alpha_factor);
        let b = apply_alpha_channel(style.foreground.b, BG_B, alpha_factor);
        cursor_x = render_text_sized(
            pixels,
            font,
            glyph_cache,
            text,
            cursor_x,
            y_offset,
            r,
            g,
            b,
            FONT_SIZE,
            LINE_HEIGHT,
            WIDTH - PADDING_X,
        );
    }
}

fn render_text(
    pixels: &mut [u8],
    font: &Font,
    glyph_cache: &mut GlyphCache,
    text: &str,
    start_x: usize,
    y_offset: usize,
    r: u8,
    g: u8,
    b: u8,
) {
    render_text_returning_x(pixels, font, glyph_cache, text, start_x, y_offset, r, g, b);
}

fn render_text_returning_x(
    pixels: &mut [u8],
    font: &Font,
    glyph_cache: &mut GlyphCache,
    text: &str,
    start_x: usize,
    y_offset: usize,
    r: u8,
    g: u8,
    b: u8,
) -> usize {
    render_text_sized(
        pixels,
        font,
        glyph_cache,
        text,
        start_x,
        y_offset,
        r,
        g,
        b,
        FONT_SIZE,
        LINE_HEIGHT,
        WIDTH - PADDING_X,
    )
}

fn render_text_right_aligned(
    pixels: &mut [u8],
    font: &Font,
    glyph_cache: &mut GlyphCache,
    text: &str,
    right_x: usize,
    y_offset: usize,
    r: u8,
    g: u8,
    b: u8,
) {
    let total_width = measure_text_width(glyph_cache, font, text, FONT_SIZE);
    let start_x = right_x.saturating_sub(total_width);
    render_text(pixels, font, glyph_cache, text, start_x, y_offset, r, g, b);
}

fn glyph_advance(glyph_cache: &mut GlyphCache, font: &Font, ch: char, font_size: f32) -> usize {
    cached_glyph(glyph_cache, font, ch, font_size).advance_width
}

fn cached_glyph<'a>(
    glyph_cache: &'a mut GlyphCache,
    font: &Font,
    ch: char,
    font_size: f32,
) -> &'a CachedGlyph {
    let key = GlyphCacheKey {
        ch,
        font_size_bits: font_size.to_bits(),
    };

    match glyph_cache.glyphs.entry(key) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => {
            let (metrics, bitmap) = font.rasterize(ch, font_size);
            entry.insert(CachedGlyph {
                width: metrics.width,
                height: metrics.height,
                xmin: metrics.xmin,
                ymin: metrics.ymin,
                advance_width: metrics.advance_width as usize,
                bitmap: bitmap.into_boxed_slice(),
            })
        }
    }
}

fn draw_glyph(
    pixels: &mut [u8],
    glyph: &CachedGlyph,
    cursor_x: usize,
    y_offset: usize,
    line_height: usize,
    r: u8,
    g: u8,
    b: u8,
) {
    let glyph_x = cursor_x as i32 + glyph.xmin;
    let glyph_y = y_offset as i32 + (line_height as i32 - 4) - glyph.height as i32 - glyph.ymin;

    let gy_start = (-glyph_y).clamp(0, glyph.height as i32) as usize;
    let gy_end = (HEIGHT as i32 - glyph_y).clamp(0, glyph.height as i32) as usize;
    let gx_start = (-glyph_x).clamp(0, glyph.width as i32) as usize;
    let gx_end = (WIDTH as i32 - glyph_x).clamp(0, glyph.width as i32) as usize;

    for gy in gy_start..gy_end {
        let py = (glyph_y + gy as i32) as usize;
        let bitmap_row = gy * glyph.width;
        let pixel_row = py * WIDTH;

        for gx in gx_start..gx_end {
            let coverage = glyph.bitmap[bitmap_row + gx];
            if coverage == 0 {
                continue;
            }

            let px = (glyph_x + gx as i32) as usize;
            let idx = (pixel_row + px) * 4;

            if coverage == 255 {
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
            } else {
                let cov = coverage as u16;
                let inv = 255 - cov;
                pixels[idx] = ((r as u16 * cov + pixels[idx] as u16 * inv) / 255) as u8;
                pixels[idx + 1] = ((g as u16 * cov + pixels[idx + 1] as u16 * inv) / 255) as u8;
                pixels[idx + 2] = ((b as u16 * cov + pixels[idx + 2] as u16 * inv) / 255) as u8;
            }
        }
    }
}

fn trim_line_ending(line: &str) -> &str {
    line.strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .or_else(|| line.strip_suffix('\r'))
        .unwrap_or(line)
}

fn encode_png(pixels: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(WIDTH * HEIGHT / 2);
    {
        let mut encoder = png::Encoder::new(&mut buf, WIDTH as u32, HEIGHT as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fastest);
        let mut writer = encoder.write_header().expect("PNG header write failed");
        writer
            .write_image_data(pixels)
            .expect("PNG data write failed");
    }
    buf
}
