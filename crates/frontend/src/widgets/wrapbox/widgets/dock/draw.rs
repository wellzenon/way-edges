use std::collections::HashMap;

use cairo::{Context, ImageSurface, SurfaceType};
use config::def::widgets::wrapbox::dock::{DockConfig, ShowTitles};
use cosmic_text::{CacheKey, Color, FontSystem, SwashCache};
use util::color::cairo_set_color;

use crate::widgets::wrapbox::widgets::dock::layout::{DockLayout, MarginsF64};

pub fn paint(
    surface: &ImageSurface,
    layout: &DockLayout,
    config: &DockConfig,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    glyph_cache: &mut HashMap<CacheKey, ImageSurface>,
) {
    let font_size = config.font_size as f64;
    let border_width = config.border_width as f64;
    let margins = MarginsF64 {
        left: config.margins.left as f64,
        right: config.margins.right as f64,
        top: config.margins.top as f64,
        bottom: config.margins.bottom as f64,
    };

    let workspace_titles = config.workspace_titles;

    let item_border_width = config.window_button.border_width as f64;
    let icon_size = config.window_button.icon_size as f64;
    let show_titles = config.window_button.show_titles;
    let title_width = config.window_button.title_width as f64;
    let item_gap = config.window_button.gap as f64;
    let item_margins = MarginsF64 {
        left: config.window_button.margins.left as f64,
        right: config.window_button.margins.right as f64,
        top: config.window_button.margins.top as f64,
        bottom: config.window_button.margins.bottom as f64,
    };
    let item_wrap_titles = config.window_button.wrap_titles;
    let item_font_size = config.window_button.font_size as f32;
    let item_line_height = item_font_size as f64 * config.window_button.line_height;
    let item_font = match &config.window_button.font_family {
        cosmic_text::FamilyOwned::Name(nome) => nome.as_str(),
        cosmic_text::FamilyOwned::Serif => "serif",
        cosmic_text::FamilyOwned::SansSerif => "sans-serif",
        cosmic_text::FamilyOwned::Monospace => "monospace",
        cosmic_text::FamilyOwned::Cursive => "cursive",
        cosmic_text::FamilyOwned::Fantasy => "fantasy",
    };

    let item_height = item_border_width * 2.0 + item_margins.top + item_margins.bottom + icon_size;

    let height = border_width * 2.0 + margins.top + margins.bottom + item_height;

    let cr = Context::new(surface).expect("Falha ao criar contexto Cairo");

    let font = match &config.font_family {
        cosmic_text::FamilyOwned::Name(nome) => nome.as_str(),
        cosmic_text::FamilyOwned::Serif => "serif",
        cosmic_text::FamilyOwned::SansSerif => "sans-serif",
        cosmic_text::FamilyOwned::Monospace => "monospace",
        cosmic_text::FamilyOwned::Cursive => "cursive",
        cosmic_text::FamilyOwned::Fantasy => "fantasy",
    };

    // 2. DESENHO DO ÍCONE (Tratamento Híbrido SVG/PNG)
    let has_icon = show_titles != ShowTitles::Only;

    // Encapsulate structural pathing to reduce boilerplate in the main drawing loop
    fn draw_rounded_rect(
        cr: &Context,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        radius: f64,
        border_width: f64,
        border_color: Color,
        bg_color: Color,
    ) {
        let draw_path = |x: f64, y: f64, w: f64, h: f64, r: f64| {
            let degrees = std::f64::consts::PI / 180.0;
            cr.new_sub_path();
            cr.arc(x + w - r, y + r, r, -90.0 * degrees, 0.0 * degrees);
            cr.arc(x + w - r, y + h - r, r, 0.0 * degrees, 90.0 * degrees);
            cr.arc(x + r, y + h - r, r, 90.0 * degrees, 180.0 * degrees);
            cr.arc(x + r, y + r, r, 180.0 * degrees, 270.0 * degrees);
            cr.close_path();
        };

        draw_path(x, y, w, h, radius);
        cairo_set_color(&cr, bg_color);
        cr.fill().unwrap();

        if border_width > 0.0 {
            let half_border = border_width / 2.0;
            draw_path(
                x + half_border,
                y + half_border,
                w - border_width,
                h - border_width,
                radius - half_border,
            );
            cr.set_line_width(border_width);
            cairo_set_color(&cr, border_color);
            cr.stroke().unwrap();
        } else {
            cr.new_path();
        }
    }

    for ws in &layout.workspaces {
        let (ws_fg_color, ws_bg_color) = if ws.is_focused {
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

        if workspace_titles {
            if let Some(surface) = &ws.tag_surface {
                let tag_x = ws.x + border_width + margins.left as f64;
                let tag_y = ws.y + (ws.height - font_size) / 2.0;

                cr.save().unwrap();
                cairo_set_color(&cr, ws_fg_color);
                cr.mask_surface(surface, tag_x, tag_y).unwrap();
                cr.restore().unwrap();
            }
        }

        for item in &ws.items {
            if item.app_id.is_none() {
                continue;
            };

            let bg_color = if item.is_focused {
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
            let icon_x = item.x + item_border_width + item_margins.left as f64;
            let icon_y = item.y + item_border_width + item_margins.top as f64;

            let icon_width = if let Some(icon_surface) = &item.icon_surface {
                let icon_opacity = if item.is_focused {
                    config.window_button.active_icon_opacity
                } else {
                    config.window_button.icon_opacity
                };

                let base_color = if item.is_focused {
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

                cr.save().unwrap();
                if icon_surface.type_() == SurfaceType::Image {
                    cr.set_source_surface(&icon_surface, icon_x, icon_y)
                        .unwrap();
                    cr.paint().unwrap();
                } else {
                    cairo_set_color(&cr, text_color);
                    cr.mask_surface(&icon_surface, icon_x, icon_y).unwrap();
                }
                cr.restore().unwrap();

                icon_surface.width() as f64
            } else {
                0.0
            };

            let has_title = match &config.window_button.show_titles {
                ShowTitles::Always | ShowTitles::Only => true,
                ShowTitles::Focused if item.is_focused => true,
                _ => false,
            };

            // skip if not showing titles
            if !has_title {
                continue;
            }

            // 3. ETIQUETA DO TÍTULO (Cosmic-Text + Clipping Fixo)
            let title_x = icon_x
                + if has_icon {
                    icon_width + item_margins.left
                } else {
                    0.0
                };

            let title_y = icon_y;

            let title = item.title.as_deref().unwrap_or("");
            if title.is_empty() || title_width <= 0.0 {
                continue;
            }
            cr.save().unwrap();

            // Aplica a máscara de clipping para cortar caracteres sobressalentes
            cr.rectangle(title_x, title_y, title_width, icon_size);
            cr.clip();

            // Configuração do Buffer abstrato de Texto
            let mut buffer = cosmic_text::Buffer::new(
                font_system,
                cosmic_text::Metrics::new(item_font_size, item_line_height as f32),
            );

            let buffer_width = if item_wrap_titles {
                Some(title_width as f32)
            } else {
                None
            };

            buffer.set_size(font_system, buffer_width, Some(icon_size as f32));

            let attrs =
                cosmic_text::Attrs::new().family(config.window_button.font_family.as_family());
            buffer.set_text(font_system, title, &attrs, cosmic_text::Shaping::Advanced);
            buffer.shape_until_scroll(font_system, true);

            let max_lines = if item_wrap_titles {
                (icon_size / item_line_height).floor() as usize
            } else {
                1
            };

            if max_lines <= 0 {
                continue;
            }

            let actual_lines = buffer.layout_runs().count();
            let visible_lines = actual_lines.min(max_lines);

            if visible_lines <= 0 {
                continue;
            }
            // Calcula área residual do eixo Y para centralização do bloco de texto
            let total_text_height = visible_lines as f64 * item_line_height;
            let center_offset_y = (icon_size - total_text_height) / 2.0;

            let text_color = if item.is_focused {
                config.window_button.active_fg_color
            } else {
                config.window_button.fg_color
            };

            // Rasterização limitando a iteração apenas às linhas qualificadas
            for run in buffer.layout_runs().take(visible_lines) {
                for glyph in run.glyphs.iter() {
                    let line_start_x = title_x as f32;
                    let line_start_y = (title_y + center_offset_y) as f32 + run.line_y;

                    let physical_glyph = glyph.physical((line_start_x, line_start_y), 1.0);

                    let Some(image) = swash_cache.get_image(font_system, physical_glyph.cache_key)
                    else {
                        continue;
                    };

                    if image.data.is_empty() || image.content != cosmic_text::SwashContent::Mask {
                        continue;
                    }

                    let x = (physical_glyph.x + image.placement.left) as f64;
                    let y = (physical_glyph.y - image.placement.top) as f64;

                    // 3. Performance: Busca no cache O(1) ou instanciação (Lazy Initialization)
                    let glyph_surface =
                        glyph_cache
                            .entry(physical_glyph.cache_key)
                            .or_insert_with(|| {
                                let w_i32 = image.placement.width as i32;
                                let h_i32 = image.placement.height as i32;

                                let mut surface =
                                    ImageSurface::create(cairo::Format::A8, w_i32, h_i32)
                                        .expect("Falha ao alocar superfície para o glyph");

                                // Escopo delimitado para liberar o lock de 'data' antes do mark_dirty()
                                {
                                    let stride = surface.stride() as usize;
                                    let mut data = surface
                                        .data()
                                        .expect("Falha ao obter buffer da superfície");
                                    let src = &image.data;
                                    let w = image.placement.width as usize;

                                    for row in 0..(image.placement.height as usize) {
                                        let src_start = row * w;
                                        let dest_start = row * stride;

                                        // Cópia direta minimizando checagens de limite internas
                                        data[dest_start..dest_start + w]
                                            .copy_from_slice(&src[src_start..src_start + w]);
                                    }
                                }

                                surface.mark_dirty();
                                surface
                            });

                    // 4. Desenho final utilizando a superfície cacheada
                    cairo_set_color(&cr, text_color);
                    cr.mask_surface(glyph_surface, x.round(), y.round())
                        .unwrap();
                }
            }
            cr.restore().unwrap();
        }
    }
}
