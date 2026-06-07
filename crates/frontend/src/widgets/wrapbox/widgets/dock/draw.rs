use std::collections::HashMap;

use backend::dock::icons::IconKey;
use cairo::{Context, ImageSurface, Rectangle, SurfaceType};
use config::def::widgets::wrapbox::dock::{DockConfig, ShowTitles};
use cosmic_text::{CacheKey, Color, FontSystem, SwashCache};
use util::color::cairo_set_color;

use crate::widgets::wrapbox::widgets::dock::layout::DockLayout;

pub fn paint(
    surface: &ImageSurface,
    layout: &DockLayout,
    config: &DockConfig,
    icon_surface_cache: &HashMap<IconKey, ImageSurface>,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    glyph_cache: &mut HashMap<CacheKey, ImageSurface>,
) {
    let is_vertical = layout.is_vertical;

    let font_size = config.font_size;
    let workspace_titles = config.workspace_titles;

    let show_titles = config.window_button.show_titles;
    let title_width = config.window_button.title_width as f64;
    let item_wrap_titles = config.window_button.wrap_titles;
    let item_font_size = config.window_button.font_size;
    let item_line_height = config.window_button.line_height;

    let icon_size = config.window_button.icon_size as f64;
    let has_icon = show_titles != ShowTitles::Only;

    let cr = Context::new(surface).expect("Failed to create Cairo context");

    fn draw_rounded_rect(
        cr: &Context,
        rect: &Rectangle,
        radius: f64,
        border_width: f64,
        border_color: Color,
        bg_color: Color,
    ) {
        let (x, y, w, h) = (rect.x(), rect.y(), rect.width(), rect.height());

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

    fn paint_text(
        cr: &Context,
        text: &str,
        rect: &Rectangle,
        color: Color,
        font_size: i32,
        line_height: f64,
        font_family: &cosmic_text::Family,
        wrap: bool,
        align_center: bool,
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
        glyph_cache: &mut HashMap<CacheKey, ImageSurface>,
    ) -> () {
        let (x, y, w_limit, h_limit) = (rect.x(), rect.y(), rect.width(), rect.height());

        if text.is_empty() || w_limit <= 0.0 {
            return;
        }

        cr.save().unwrap();
        cr.rectangle(x, y, w_limit, h_limit);
        cr.clip();

        let font_height = font_size as f64 * line_height;
        let mut buffer = cosmic_text::Buffer::new(
            font_system,
            cosmic_text::Metrics::new(font_size as f32, font_height as f32),
        );

        let buffer_width = if wrap { Some(w_limit as f32) } else { None };
        buffer.set_size(font_system, buffer_width, Some(h_limit as f32));

        let attrs = cosmic_text::Attrs::new().family(*font_family);
        buffer.set_text(font_system, text, &attrs, cosmic_text::Shaping::Advanced);
        buffer.shape_until_scroll(font_system, true);

        let max_lines = if wrap {
            ((h_limit / font_height).round() as usize).max(1)
        } else {
            1
        };

        let visible_lines = buffer.layout_runs().count().min(max_lines);
        if visible_lines > 0 {
            let total_text_height = visible_lines as f64 * font_height;
            let center_offset_y = (h_limit - total_text_height) / 2.0;

            for run in buffer.layout_runs().take(visible_lines) {
                let center_offset_x = if align_center {
                    (w_limit - run.line_w as f64) / 2.0
                } else {
                    0.0
                };

                for glyph in run.glyphs.iter() {
                    let line_start_x = (x + center_offset_x) as f32;
                    let line_start_y = (y + center_offset_y) as f32 + run.line_y;

                    let physical_glyph = glyph.physical((line_start_x, line_start_y), 1.0);
                    let Some(image) = swash_cache.get_image(font_system, physical_glyph.cache_key)
                    else {
                        continue;
                    };

                    if image.data.is_empty() || image.content != cosmic_text::SwashContent::Mask {
                        continue;
                    }

                    let gx = (physical_glyph.x + image.placement.left) as f64;
                    let gy = (physical_glyph.y - image.placement.top) as f64;

                    let glyph_surface =
                        glyph_cache
                            .entry(physical_glyph.cache_key)
                            .or_insert_with(|| {
                                let w_i32 = image.placement.width as i32;
                                let h_i32 = image.placement.height as i32;

                                let mut surface =
                                    ImageSurface::create(cairo::Format::A8, w_i32, h_i32)
                                        .expect("Falha ao alocar superfície do glifo");

                                {
                                    let stride = surface.stride() as usize;
                                    let mut data = surface.data().unwrap();
                                    let src = &image.data;
                                    let w = image.placement.width as usize;

                                    for row in 0..(image.placement.height as usize) {
                                        let src_start = row * w;
                                        let dest_start = row * stride;
                                        data[dest_start..dest_start + w]
                                            .copy_from_slice(&src[src_start..src_start + w]);
                                    }
                                }
                                surface.mark_dirty();
                                surface
                            });

                    cairo_set_color(cr, color);
                    cr.mask_surface(glyph_surface, gx.round(), gy.round())
                        .unwrap();
                }
            }
        }
        cr.restore().unwrap();
    }

    for ws in &layout.workspaces {
        let (ws_fg_color, ws_bg_color) = if ws.is_focused {
            (config.active_fg_color, config.active_bg_color)
        } else {
            (config.fg_color, config.bg_color)
        };

        draw_rounded_rect(
            &cr,
            &ws.rect,
            config.border_radius as f64,
            config.border_width as f64,
            config.border_color,
            ws_bg_color,
        );

        if workspace_titles && ws.tag_name.is_some() && ws.tag_rect.width() > 0.0 {
            paint_text(
                &cr,
                ws.tag_name.as_deref().unwrap(),
                &ws.tag_rect,
                ws_fg_color,
                font_size,
                1.0,
                &config.font_family.as_family(),
                false,
                true,
                font_system,
                swash_cache,
                glyph_cache,
            );
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

            // Window Button paint
            draw_rounded_rect(
                &cr,
                &item.rect,
                config.window_button.border_radius as f64,
                config.window_button.border_width as f64,
                config.window_button.border_color,
                bg_color,
            );

            // 2. DESENHO DO ÍCONE (Tratamento Híbrido SVG/PNG)

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

            if has_icon && item.icon_rect.width() > 0.0 {
                if item.has_resolved_icon {
                    if let Some(icon_surface) = icon_surface_cache.get(&item.icon_key) {
                        cr.save().unwrap();
                        if icon_surface.type_() == SurfaceType::Image {
                            cr.set_source_surface(
                                icon_surface,
                                item.icon_rect.x(),
                                item.icon_rect.y(),
                            )
                            .unwrap();
                            cr.paint_with_alpha(icon_opacity).unwrap();
                        } else {
                            cairo_set_color(&cr, text_color);
                            cr.mask_surface(icon_surface, item.icon_rect.x(), item.icon_rect.y())
                                .unwrap();
                        }
                        cr.restore().unwrap();
                    }
                } else {
                    let char_icon: String = if !item.fallback_char.is_empty() {
                        item.fallback_char.to_uppercase().clone()
                    } else {
                        item.title
                            .as_deref()
                            .and_then(|t| t.chars().next())
                            .unwrap_or('?')
                            .to_uppercase()
                            .to_string()
                    };

                    paint_text(
                        &cr,
                        &char_icon,
                        &item.icon_rect,
                        base_color,
                        icon_size as i32,
                        1.0,
                        &config.window_button.font_family.as_family(),
                        false,
                        true,
                        font_system,
                        swash_cache,
                        glyph_cache,
                    );
                }
            }

            let has_title = match &config.window_button.show_titles {
                ShowTitles::Always | ShowTitles::Only => true,
                ShowTitles::Focused if item.is_focused => true,
                _ => false,
            };

            // skip if not showing titles
            if !has_title {
                continue;
            }

            let title = item.title.as_deref().unwrap_or("");

            if title.is_empty() || title_width <= 0.0 {
                continue;
            }

            paint_text(
                &cr,
                title,
                &item.title_rect,
                base_color,
                item_font_size,
                item_line_height,
                &config.window_button.font_family.as_family(),
                item_wrap_titles,
                is_vertical,
                font_system,
                swash_cache,
                glyph_cache,
            );
        }
    }
}
