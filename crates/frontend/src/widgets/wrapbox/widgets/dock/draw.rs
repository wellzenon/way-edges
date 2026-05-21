use std::collections::HashMap;

use backend::niri::{IconStatus, ICON_CACHE};
use cairo::{Context, Format, ImageSurface};
use config::def::widgets::wrapbox::dock::{DockConfig, ShowTitles};
use cosmic_text::{Color, FontSystem, SwashCache};
use util::color::cairo_set_color;

use crate::widgets::wrapbox::widgets::dock::layout::DockLayout;

fn load_icon(app_id: &str, cache: &mut HashMap<String, ImageSurface>) -> Option<ImageSurface> {
    // 1. Hit do Cache Frontend O(1)
    if let Some(surface) = cache.get(app_id) {
        return Some(surface.clone());
    }

    // 2. Consulta ao Cache Global do Backend
    let backend_cache = ICON_CACHE.read().unwrap();
    let status = backend_cache.get(app_id)?;

    match status {
        IconStatus::Ready {
            pixels,
            width,
            height,
            ..
        } => {
            let mut surface = ImageSurface::create(Format::ARgb32, *width, *height).ok()?;

            {
                let stride = surface.stride() as usize;
                let mut data = surface.data().ok()?;
                let w_bytes = (*width * 4) as usize;

                for (dest_row, src_row) in data
                    .chunks_exact_mut(stride)
                    .zip(pixels.chunks_exact(w_bytes))
                {
                    dest_row[..w_bytes].copy_from_slice(src_row);
                }
            }
            surface.mark_dirty();

            cache.insert(app_id.to_string(), surface.clone());
            Some(surface)
        }
        IconStatus::Loading | IconStatus::NotFound => None,
    }
}

pub fn paint(
    surface: &ImageSurface,
    layout: &DockLayout,
    config: &DockConfig,
    cache: &mut HashMap<String, ImageSurface>,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
) {
    let cr = Context::new(surface).expect("Falha ao criar contexto Cairo");

    let ws_font_size = config.font_size as f64;
    let ws_margins = &config.margins;
    let ws_font = match &config.window_button.font_family {
        cosmic_text::FamilyOwned::Name(nome) => nome.as_str(),
        cosmic_text::FamilyOwned::Serif => "serif",
        cosmic_text::FamilyOwned::SansSerif => "sans-serif",
        cosmic_text::FamilyOwned::Monospace => "monospace",
        cosmic_text::FamilyOwned::Cursive => "cursive",
        cosmic_text::FamilyOwned::Fantasy => "fantasy",
    };

    // 2. DESENHO DO ÍCONE (Tratamento Híbrido SVG/PNG)
    let icon_size = config.window_button.icon_size as f64;
    let item_margins = &config.window_button.margins;
    let ws_show_titles = config.workspace_titles;
    let item_font = match &config.window_button.font_family {
        cosmic_text::FamilyOwned::Name(nome) => nome.as_str(),
        cosmic_text::FamilyOwned::Serif => "serif",
        cosmic_text::FamilyOwned::SansSerif => "sans-serif",
        cosmic_text::FamilyOwned::Monospace => "monospace",
        cosmic_text::FamilyOwned::Cursive => "cursive",
        cosmic_text::FamilyOwned::Fantasy => "fantasy",
    };
    let has_icon = config.window_button.show_titles != ShowTitles::Only;

    let item_font_size = config.window_button.font_size as f64;
    let item_line_height = item_font_size * config.window_button.line_height;
    let wrap_titles = config.window_button.wrap_titles;
    let title_width = config.window_button.title_width as f64;
    let title_height = icon_size;
    let item_gap = config.window_button.gap as f64;

    // Encapsulate structural pathing to reduce boilerplate in the main drawing loop
    let draw_rounded_rect = |cr: &Context,
                             x: f64,
                             y: f64,
                             w: f64,
                             h: f64,
                             radius: f64,
                             border_width: f64,
                             border_color: Color,
                             bg_color: Color| {
        let degrees = std::f64::consts::PI / 180.0;

        cairo_set_color(&cr, bg_color);

        cr.new_sub_path();
        cr.arc(
            x + w - radius,
            y + radius,
            radius,
            -90.0 * degrees,
            0.0 * degrees,
        );
        cr.arc(
            x + w - radius,
            y + h - radius,
            radius,
            0.0 * degrees,
            90.0 * degrees,
        );
        cr.arc(
            x + radius,
            y + h - radius,
            radius,
            90.0 * degrees,
            180.0 * degrees,
        );
        cr.arc(
            x + radius,
            y + radius,
            radius,
            180.0 * degrees,
            270.0 * degrees,
        );
        cr.close_path();

        cr.fill_preserve().unwrap();

        if border_width > 0.0 {
            cairo_set_color(&cr, border_color);
            cr.set_line_width(border_width);
            cr.stroke().unwrap();
        } else {
            cr.new_path();
        }
    };

    for ws in &layout.workspaces {
        let (ws_fg_color, ws_bg_color) = if ws.is_active {
            (config.active_fg_color, config.active_bg_color)
        } else {
            (config.fg_color, config.bg_color)
        };

        cr.save().unwrap();

        // Workspace Divisor paint
        draw_rounded_rect(
            &cr,
            ws.x,
            ws.y,
            ws.width,
            ws.height,
            config.border_radius as f64,
            config.border_width as f64,
            config.border_color,
            ws_bg_color,
        );

        cr.restore().unwrap();

        if let Some(ws_name) = &ws.name {
            if ws_show_titles {
                cr.save().unwrap();

                cairo_set_color(&cr, ws_fg_color);
                cr.select_font_face(ws_font, cairo::FontSlant::Normal, cairo::FontWeight::Bold);

                cr.set_font_size(ws_font_size);

                if let Ok(extents) = cr.text_extents(ws_name) {
                    let tag_x = ws.x + ws_margins.left as f64;
                    let tag_y = ws.y + (ws.height - extents.height()) / 2.0 - extents.y_bearing();

                    cr.move_to(tag_x, tag_y);
                    cr.show_text(ws_name).unwrap();
                }
                cr.restore().unwrap();
            }
        }

        for item in &ws.items {
            let win = &item.window;

            let Some(app_id) = &win.app_id else {
                continue;
            };

            let bg_color = if win.is_focused {
                config.window_button.active_bg_color
            } else {
                config.window_button.bg_color
            };

            cr.save().unwrap();

            // Window Button paint
            draw_rounded_rect(
                &cr,
                item.x,
                item.y,
                item.width,
                item.height,
                config.window_button.border_radius as f64,
                config.window_button.border_width as f64,
                config.window_button.border_color,
                bg_color,
            );

            cr.restore().unwrap();

            // 2. DESENHO DO ÍCONE (Tratamento Híbrido SVG/PNG)
            let icon_x = item.x + item_margins.left as f64;
            let icon_y = item.y + item_margins.top as f64;

            if has_icon {
                let icon_opacity = if win.is_focused {
                    config.window_button.active_icon_opacity
                } else {
                    config.window_button.icon_opacity
                };

                match load_icon(&app_id, cache) {
                    // assumindo que app_id já foi extraído como &str
                    Some(icon_surface) => {
                        cr.save().unwrap();
                        let scale_x = icon_size / icon_surface.width() as f64;
                        let scale_y = icon_size / icon_surface.height() as f64;
                        cr.translate(icon_x, icon_y);
                        cr.scale(scale_x, scale_y);
                        cr.set_source_surface(&icon_surface, 0.0, 0.0).unwrap();
                        cr.paint_with_alpha(icon_opacity).unwrap();
                        cr.restore().unwrap();
                    }
                    None => {
                        let fallback = config.window_button.icon_fallback.as_deref();

                        // Se chegamos aqui, o backend não tem imagem. O fallback deve ser tratado
                        // puramente como texto (ex: um caractere NerdFont) ou a inicial do app.
                        let identifier =
                            if let Some(char_str) = fallback.filter(|f| f.chars().count() <= 4) {
                                // String curta: assume-se que seja um ícone/caractere de fonte (ex: "󰣆")
                                char_str.to_string()
                            } else {
                                // String longa (era um caminho que falhou no backend) ou ausente:
                                // Extrai a primeira letra do app_id ou do título da janela.
                                let win_title = win
                                    .app_id
                                    .as_deref()
                                    .unwrap_or(win.title.as_deref().unwrap_or("?"));
                                win_title
                                    .split('.')
                                    .last()
                                    .unwrap_or("?")
                                    .chars()
                                    .next()
                                    .unwrap_or('?')
                                    .to_uppercase()
                                    .to_string()
                            };

                        cr.save().unwrap();

                        let base_color = if win.is_focused {
                            config.window_button.active_fg_color
                        } else {
                            config.window_button.fg_color
                        };

                        let text_color = cosmic_text::Color::rgba(
                            base_color.r(),
                            base_color.g(),
                            base_color.b(),
                            (icon_opacity * 255.0) as u8,
                        );

                        cairo_set_color(&cr, text_color);

                        cr.select_font_face(
                            item_font,
                            cairo::FontSlant::Normal,
                            cairo::FontWeight::Bold,
                        );
                        cr.set_font_size(icon_size);

                        if let Ok(extents) = cr.text_extents(&identifier) {
                            let text_x =
                                icon_x + (icon_size - extents.width()) / 2.0 - extents.x_bearing();
                            let text_y =
                                icon_y + (icon_size - extents.height()) / 2.0 - extents.y_bearing();

                            cr.move_to(text_x, text_y);
                            cr.show_text(&identifier).unwrap();
                        }

                        cr.restore().unwrap();
                    }
                }
            };

            // skip if not showing titles
            match &config.window_button.show_titles {
                ShowTitles::Always | ShowTitles::Only => {}
                ShowTitles::Focused if win.is_focused => {}
                _ => continue,
            };

            // 3. ETIQUETA DO TÍTULO (Cosmic-Text + Clipping Fixo)
            let title_x = icon_x + if has_icon { icon_size + item_gap } else { 0.0 };
            let title_y = icon_y;

            let titulo = win.title.as_deref().unwrap_or("");
            if !titulo.is_empty() && title_width > 0.0 {
                cr.save().unwrap();

                // Aplica a máscara de clipping para cortar caracteres sobressalentes
                cr.rectangle(title_x, title_y, title_width, title_height);
                cr.clip();

                // Configuração do Buffer abstrato de Texto
                let mut buffer = cosmic_text::Buffer::new(
                    font_system,
                    cosmic_text::Metrics::new(item_font_size as f32, item_line_height as f32),
                );

                let buffer_width = if wrap_titles {
                    Some(title_width as f32)
                } else {
                    None
                };

                buffer.set_size(font_system, buffer_width, Some(title_height as f32));

                let attrs =
                    cosmic_text::Attrs::new().family(config.window_button.font_family.as_family());
                buffer.set_text(font_system, titulo, &attrs, cosmic_text::Shaping::Advanced);
                buffer.shape_until_scroll(font_system, true);

                let max_lines = if wrap_titles {
                    (title_height / item_line_height).floor() as usize
                } else {
                    1
                };

                if max_lines > 0 {
                    let actual_lines = buffer.layout_runs().count();
                    let visible_lines = actual_lines.min(max_lines);

                    if visible_lines > 0 {
                        // Calcula área residual do eixo Y para centralização do bloco de texto
                        let total_text_height = visible_lines as f64 * item_line_height;
                        let center_offset_y = (title_height - total_text_height) / 2.0;

                        let text_color = if win.is_focused {
                            config.window_button.active_fg_color
                        } else {
                            config.window_button.fg_color
                        };

                        // Rasterização limitando a iteração apenas às linhas qualificadas
                        for run in buffer.layout_runs().take(visible_lines) {
                            for glyph in run.glyphs.iter() {
                                let line_start_x = title_x as f32;
                                // Adição de center_offset_y calibra as linhas no centro do eixo
                                let line_start_y = (title_y + center_offset_y) as f32 + run.line_y;

                                let physical_glyph =
                                    glyph.physical((line_start_x, line_start_y), 1.0);

                                if let Some(image) =
                                    swash_cache.get_image(font_system, physical_glyph.cache_key)
                                {
                                    let x = (physical_glyph.x + image.placement.left) as f64;
                                    let y = (physical_glyph.y - image.placement.top) as f64;

                                    if !image.data.is_empty()
                                        && image.content == cosmic_text::SwashContent::Mask
                                    {
                                        let glyph_width = image.placement.width as i32;
                                        let glyph_height = image.placement.height as i32;

                                        if let Ok(mut glyph_surface) = ImageSurface::create(
                                            Format::A8,
                                            glyph_width,
                                            glyph_height,
                                        ) {
                                            let stride = glyph_surface.stride() as usize;

                                            if let Ok(mut data) = glyph_surface.data() {
                                                let src = &image.data;
                                                let w = glyph_width as usize;
                                                let h = glyph_height as usize;

                                                for row in 0..h {
                                                    let src_start = row * w;
                                                    let src_end = src_start + w;
                                                    let dest_start = row * stride;
                                                    let dest_end = dest_start + w;

                                                    if src_end <= src.len()
                                                        && dest_end <= data.len()
                                                    {
                                                        data[dest_start..dest_end].copy_from_slice(
                                                            &src[src_start..src_end],
                                                        );
                                                    }
                                                }
                                            }

                                            glyph_surface.mark_dirty();
                                            cairo_set_color(&cr, text_color);
                                            cr.mask_surface(&glyph_surface, x.round(), y.round())
                                                .unwrap();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                cr.restore().unwrap();
            }
        }
    }
}
