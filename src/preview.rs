use fontdue::{Font, FontSettings};
use syntect::{
    easy::HighlightLines,
    highlighting::Style,
    parsing::SyntaxReference,
    util::LinesWithEndings,
};

use crate::state::AppState;

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

pub fn load_font() -> Font {
    let font_data = include_bytes!("../font/DMMono-Regular.ttf");
    Font::from_bytes(font_data as &[u8], FontSettings::default())
        .expect("failed to load embedded font")
}

pub fn generate_preview(state: &AppState, content: &str, extension: Option<&str>) -> Vec<u8> {
    let mut pixels = vec![0u8; WIDTH * HEIGHT * 4];

    // Fill background
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[0] = BG_R;
        pixel[1] = BG_G;
        pixel[2] = BG_B;
        pixel[3] = 255;
    }

    if content.is_empty() {
        return encode_png(&pixels);
    }

    let syntax = resolve_syntax(state, extension);
    let lines: Vec<&str> = LinesWithEndings::from(content).take(MAX_LINES + 1).collect();
    let has_more = lines.len() > MAX_LINES;
    let visible_lines = if has_more { MAX_LINES } else { lines.len() };

    let font = &state.font;

    // Render each line
    let mut highlighter = syntax.map(|s| HighlightLines::new(s, state.theme.as_ref()));

    for (line_idx, &line) in lines.iter().take(visible_lines).enumerate() {
        let y_offset = PADDING_Y + line_idx * LINE_HEIGHT;
        if y_offset + LINE_HEIGHT > HEIGHT {
            break;
        }

        let line_num = line_idx + 1;

        // Render line number (right-aligned in LINE_NUMBER_WIDTH area)
        let num_str = itoa::Buffer::new().format(line_num).to_string();
        render_text_right_aligned(
            &mut pixels,
            font,
            &num_str,
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

        for ch in text.chars() {
            if ch == '\t' {
                let space_advance = char_advance(font, ' ');
                cursor_x += space_advance * TAB_WIDTH;
                continue;
            }

            if cursor_x >= WIDTH - PADDING_X {
                break;
            }

            let (metrics, bitmap) = font.rasterize(ch, FONT_SIZE);
            let glyph_y = y_offset as i32 + (LINE_HEIGHT as i32 - 4) - metrics.height as i32 - metrics.ymin;

            for gy in 0..metrics.height {
                for gx in 0..metrics.width {
                    let coverage = bitmap[gy * metrics.width + gx];
                    if coverage == 0 {
                        continue;
                    }

                    let px = cursor_x as i32 + metrics.xmin + gx as i32;
                    let py = glyph_y + gy as i32;

                    if px < 0 || py < 0 || px as usize >= WIDTH || py as usize >= HEIGHT {
                        continue;
                    }

                    let idx = (py as usize * WIDTH + px as usize) * 4;
                    if idx + 3 >= pixels.len() {
                        continue;
                    }

                    if coverage == 255 {
                        pixels[idx] = r;
                        pixels[idx + 1] = g;
                        pixels[idx + 2] = b;
                    } else {
                        let cov = coverage as u16;
                        let inv = 255 - cov;
                        pixels[idx] = ((r as u16 * cov + pixels[idx] as u16 * inv) / 255) as u8;
                        pixels[idx + 1] =
                            ((g as u16 * cov + pixels[idx + 1] as u16 * inv) / 255) as u8;
                        pixels[idx + 2] =
                            ((b as u16 * cov + pixels[idx + 2] as u16 * inv) / 255) as u8;
                    }
                }
            }

            cursor_x += metrics.advance_width as usize;
        }
    }
}

fn render_text(
    pixels: &mut [u8],
    font: &Font,
    text: &str,
    start_x: usize,
    y_offset: usize,
    r: u8,
    g: u8,
    b: u8,
) {
    let mut cursor_x = start_x;

    for ch in text.chars() {
        if ch == '\t' {
            let space_advance = char_advance(font, ' ');
            cursor_x += space_advance * TAB_WIDTH;
            continue;
        }

        if cursor_x >= WIDTH - PADDING_X {
            break;
        }

        let (metrics, bitmap) = font.rasterize(ch, FONT_SIZE);
        let glyph_y =
            y_offset as i32 + (LINE_HEIGHT as i32 - 4) - metrics.height as i32 - metrics.ymin;

        for gy in 0..metrics.height {
            for gx in 0..metrics.width {
                let coverage = bitmap[gy * metrics.width + gx];
                if coverage == 0 {
                    continue;
                }

                let px = cursor_x as i32 + metrics.xmin + gx as i32;
                let py = glyph_y + gy as i32;

                if px < 0 || py < 0 || px as usize >= WIDTH || py as usize >= HEIGHT {
                    continue;
                }

                let idx = (py as usize * WIDTH + px as usize) * 4;
                if idx + 3 >= pixels.len() {
                    continue;
                }

                if coverage == 255 {
                    pixels[idx] = r;
                    pixels[idx + 1] = g;
                    pixels[idx + 2] = b;
                } else {
                    let cov = coverage as u16;
                    let inv = 255 - cov;
                    pixels[idx] = ((r as u16 * cov + pixels[idx] as u16 * inv) / 255) as u8;
                    pixels[idx + 1] =
                        ((g as u16 * cov + pixels[idx + 1] as u16 * inv) / 255) as u8;
                    pixels[idx + 2] =
                        ((b as u16 * cov + pixels[idx + 2] as u16 * inv) / 255) as u8;
                }
            }
        }

        cursor_x += metrics.advance_width as usize;
    }
}

fn render_text_right_aligned(
    pixels: &mut [u8],
    font: &Font,
    text: &str,
    right_x: usize,
    y_offset: usize,
    r: u8,
    g: u8,
    b: u8,
) {
    // Measure total width
    let total_width: usize = text.chars().map(|ch| char_advance(font, ch)).sum();
    let start_x = right_x.saturating_sub(total_width);
    render_text(pixels, font, text, start_x, y_offset, r, g, b);
}

fn char_advance(font: &Font, ch: char) -> usize {
    let metrics = font.metrics(ch, FONT_SIZE);
    metrics.advance_width as usize
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
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header().expect("PNG header write failed");
        writer.write_image_data(pixels).expect("PNG data write failed");
    }
    buf
}
