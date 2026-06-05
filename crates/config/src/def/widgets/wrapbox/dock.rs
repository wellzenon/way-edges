use cosmic_text::{Color, FamilyOwned};
use knus::{Decode, DecodeScalar};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer};
use util::color::parse_color;

use crate::def::shared::{
    color_translate, deserialize_family_owned, dt_family_owned, parse_family_owned, schema_color,
    schema_family_owned, NumMargins,
};

use super::Align;

#[derive(Debug, Default, Copy, Eq, Clone, DecodeScalar, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ShowTitles {
    #[default]
    Always,
    Focused,
    Never,
    Only,
}

fn icon_fallback_deserialize<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    // 1. Instrui o parser a tentar ler o valor atual como uma String opcional
    let opt_string = Option::<String>::deserialize(deserializer)?;
    match opt_string {
        Some(valor) => {
            let valor_trim = valor.trim();

            if valor_trim.ends_with(".png") || valor_trim.ends_with(".svg") {
                return Ok(Some(valor_trim.to_string()));
            }

            let tamanho = valor_trim.chars().count();
            if tamanho > 0 && tamanho <= 4 {
                return Ok(Some(valor_trim.to_string()));
            }
            Err(serde::de::Error::custom(format!(
                "Invalid icon-fallback: '{}'. Should be only one character, symbol, emoji or a path to an image file ending with .svg or .png.",
                valor_trim
            )))
        }
        None => Ok(None),
    }
}

#[derive(Debug, Clone, Decode, PartialEq, Deserialize, JsonSchema)]
pub struct WindowButton {
    #[knus(child, default = dt_family_owned(), unwrap(argument, decode_with = parse_family_owned))]
    #[serde(default = "dt_family_owned")]
    #[serde(deserialize_with = "deserialize_family_owned")]
    #[schemars(schema_with = "schema_family_owned")]
    pub font_family: FamilyOwned,

    #[knus(child, default = dt_wb_font_size(), unwrap(argument))]
    #[serde(default = "dt_wb_font_size")]
    pub font_size: i32,

    #[knus(child, default = dt_wb_line_height(), unwrap(argument))]
    #[serde(default = "dt_wb_line_height")]
    pub line_height: f64,

    #[knus(child, default = dt_wb_show_titles(), unwrap(argument))]
    #[serde(default = "dt_wb_show_titles")]
    pub show_titles: ShowTitles,

    #[knus(child, default = dt_wb_wrap_titles(), unwrap(argument))]
    #[serde(default = "dt_wb_wrap_titles")]
    pub wrap_titles: bool,

    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub icon_theme: Option<String>,

    #[knus(child, default = dt_wb_icon_size(), unwrap(argument))]
    #[serde(default = "dt_wb_icon_size")]
    pub icon_size: f64,

    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    #[serde(deserialize_with = "icon_fallback_deserialize")]
    pub icon_fallback: Option<String>,

    #[knus(child, default = dt_wb_icon_opacity(), unwrap(argument))]
    #[serde(default = "dt_wb_icon_opacity")]
    pub icon_opacity: f64,

    #[knus(child, default = dt_wb_active_icon_opacity(), unwrap(argument))]
    #[serde(default = "dt_wb_active_icon_opacity")]
    pub active_icon_opacity: f64,

    #[knus(child, default = dt_wb_title_width(), unwrap(argument))]
    #[serde(default = "dt_wb_title_width")]
    pub title_width: f64,

    #[knus(child, default = dt_wb_fg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wb_fg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub fg_color: Color,

    #[knus(child, default = dt_wb_active_fg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wb_active_fg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub active_fg_color: Color,

    #[knus(child, default = dt_wb_bg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wb_bg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub bg_color: Color,

    #[knus(child, default = dt_wb_active_bg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wb_active_bg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub active_bg_color: Color,

    #[knus(child, default = dt_wb_border_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_wb_border_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub border_color: Color,

    #[knus(child, default = dt_wb_border_width(), unwrap(argument))]
    #[serde(default = "dt_wb_border_width")]
    pub border_width: f64,

    #[knus(child, default = dt_wb_border_radius(), unwrap(argument))]
    #[serde(default = "dt_wb_border_radius")]
    pub border_radius: f64,
    #[knus(child, default = dt_wb_gap(), unwrap(argument))]
    #[serde(default = "dt_wb_gap")]
    pub gap: f64,

    #[knus(child, default = dt_margin())]
    #[serde(default = "dt_wb_margin")]
    pub margins: NumMargins,
}

impl Default for WindowButton {
    fn default() -> Self {
        Self {
            font_family: dt_family_owned(),
            font_size: dt_wb_font_size(),
            line_height: dt_wb_line_height(),
            title_width: dt_wb_title_width(),
            show_titles: dt_wb_show_titles(),
            wrap_titles: dt_wb_wrap_titles(),
            icon_theme: None,
            icon_size: dt_wb_icon_size(),
            icon_fallback: None,
            icon_opacity: dt_wb_icon_opacity(),
            active_icon_opacity: dt_wb_active_icon_opacity(),
            fg_color: dt_wb_fg_color(),
            active_fg_color: dt_wb_active_fg_color(),
            bg_color: dt_wb_bg_color(),
            active_bg_color: dt_wb_active_bg_color(),
            border_color: dt_wb_border_color(),
            border_width: dt_wb_border_width(),
            border_radius: dt_wb_border_radius(),
            gap: dt_wb_gap(),
            margins: dt_wb_margin(),
        }
    }
}

#[derive(Debug, Decode, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(deny_unknown_fields)]
pub struct DockConfig {
    #[knus(child, default, unwrap(argument))]
    #[serde(default)]
    pub grid_align: Align,

    #[knus(child, default = dt_family_owned(), unwrap(argument, decode_with = parse_family_owned))]
    #[serde(default = "dt_family_owned")]
    #[serde(deserialize_with = "deserialize_family_owned")]
    #[schemars(schema_with = "schema_family_owned")]
    pub font_family: FamilyOwned,

    #[knus(child, default = dt_font_size(), unwrap(argument))]
    #[serde(default = "dt_font_size")]
    pub font_size: i32,

    #[knus(child, default = dt_workspace_titles(), unwrap(argument))]
    #[serde(default = "dt_workspace_titles")]
    pub workspace_titles: bool,

    #[knus(child, default = dt_fg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_fg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub fg_color: Color,

    #[knus(child, default = dt_active_fg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_active_fg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub active_fg_color: Color,

    #[knus(child, default = dt_bg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_bg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub bg_color: Color,

    #[knus(child, default = dt_active_bg_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_active_bg_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub active_bg_color: Color,

    #[knus(child, default = dt_border_color(), unwrap(argument, decode_with = parse_color))]
    #[serde(default = "dt_border_color")]
    #[serde(deserialize_with = "color_translate")]
    #[schemars(schema_with = "schema_color")]
    pub border_color: Color,

    #[knus(child, default = dt_border_width(), unwrap(argument))]
    #[serde(default = "dt_border_width")]
    pub border_width: i32,

    #[knus(child, default = dt_border_radius(), unwrap(argument))]
    #[serde(default = "dt_border_radius")]
    pub border_radius: i32,

    #[knus(child, default = dt_gap(), unwrap(argument))]
    #[serde(default = "dt_gap")]
    pub gap: i32,

    #[knus(child, default = dt_margin())]
    #[serde(default = "dt_margin")]
    pub margins: NumMargins,

    #[knus(child, default = dt_window_button())]
    #[serde(default = "dt_window_button")]
    pub window_button: WindowButton,
}

impl Default for DockConfig {
    fn default() -> Self {
        Self {
            grid_align: Align::default(),
            font_family: dt_family_owned(),
            font_size: dt_font_size(),
            workspace_titles: dt_workspace_titles(),
            fg_color: dt_fg_color(),
            active_fg_color: dt_active_fg_color(),
            bg_color: dt_bg_color(),
            active_bg_color: dt_active_bg_color(),
            border_color: dt_border_color(),
            border_width: dt_border_width(),
            border_radius: dt_border_radius(),
            gap: dt_gap(),
            margins: dt_margin(),
            window_button: WindowButton::default(),
        }
    }
}

// Geração em massa das funções de fallback exigidas pelo parser
macro_rules! def_fallback {
    ($func_name:ident, $ret_type:ty, $value:expr) => {
        pub fn $func_name() -> $ret_type {
            $value
        }
    };
}

def_fallback!(dt_font_size, i32, 26);
def_fallback!(dt_workspace_titles, bool, true);
def_fallback!(dt_fg_color, Color, Color::rgba(255, 255, 255, 80));
def_fallback!(dt_active_fg_color, Color, Color::rgba(255, 255, 255, 160));
def_fallback!(dt_bg_color, Color, Color::rgba(0, 0, 0, 255));
def_fallback!(dt_active_bg_color, Color, Color::rgba(255, 255, 255, 17));
def_fallback!(dt_border_color, Color, Color::rgba(0, 0, 0, 255));
def_fallback!(dt_border_width, i32, 5);
def_fallback!(dt_border_radius, i32, 10);
def_fallback!(dt_gap, i32, 0);
def_fallback!(
    dt_margin,
    NumMargins,
    NumMargins {
        left: 15,
        right: 15,
        top: 10,
        bottom: 10
    }
);

// window_button WindowButton
def_fallback!(dt_window_button, WindowButton, WindowButton::default());
def_fallback!(dt_wb_font_size, i32, 16);
def_fallback!(dt_wb_line_height, f64, 1.4);
def_fallback!(dt_wb_title_width, f64, 60.0);
def_fallback!(dt_wb_show_titles, ShowTitles, ShowTitles::Focused);
def_fallback!(dt_wb_wrap_titles, bool, false);
def_fallback!(dt_wb_icon_size, f64, 32.0);
def_fallback!(dt_wb_icon_opacity, f64, 0.5);
def_fallback!(dt_wb_active_icon_opacity, f64, 1.0);
def_fallback!(dt_wb_fg_color, Color, Color::rgba(180, 180, 180, 160));
def_fallback!(
    dt_wb_active_fg_color,
    Color,
    Color::rgba(230, 230, 230, 255)
);
def_fallback!(dt_wb_bg_color, Color, Color::rgba(0, 0, 0, 0));
def_fallback!(dt_wb_active_bg_color, Color, Color::rgba(255, 255, 255, 17));
def_fallback!(dt_wb_border_color, Color, Color::rgba(0, 0, 0, 0));
def_fallback!(dt_wb_border_width, f64, 0.0);
def_fallback!(dt_wb_border_radius, f64, 10.0);
def_fallback!(dt_wb_gap, f64, 5.0);
def_fallback!(
    dt_wb_margin,
    NumMargins,
    NumMargins {
        left: 10,
        right: 10,
        top: 10,
        bottom: 10
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_dock_configs() {
        let kdl = r##"
wrap-box {
    edge "bottom"
    thickness 32
    length "100%"
    item "dock" {
        index 0 0
        grid-align "center-center"
        font-family "mono"
        font-size 12
        workspace-titles true
        fg-color "#ffffff"
        bg-color "#000000"
        active-fg-color "#0000ff"
        active-bg-color "#00ff00"
        border-color "#333333"
        border-width 2
        border-radius 15
        gap 8
        margins {
            left 4
            right 4
            top 4
            bottom 4
        }
        window-button {
            font-family "mono"
            font-size 12
            line-height 1.3
            title-width 48
            show-titles "always"
            wrap-titles true
            icon-theme "Adwaita"
            icon-size 24
            icon-fallback "?"
            icon-opacity 0.5
            active-icon-opacity 1
            fg-color "#ffffff"
            active-fg-color "#0000ff"
            bg-color "#000000"
            active-bg-color "#00ff00"
            border-color "#000000"
            border-width 2
            border-radius 15
            gap 4
            margins {
                left 2
                right 2
                top 2
                bottom 2
            }
        }
    }
}
"##;
        let parsed: Vec<crate::def::WidgetConf> = knus::parse("test", kdl).unwrap();
        if let crate::def::WidgetConf::WrapBox(wrap_box) = &parsed[0] {
            let config = &wrap_box.widget;
            assert_eq!(config.items.len(), 1);

            if let crate::def::widgets::wrapbox::BoxedWidget::Dock(dock_config) =
                &config.items[0].widget
            {
                assert_eq!(config.items[0].index, [0, 0]);
                assert!(matches!(dock_config.grid_align, Align::CenterCenter));
                assert_eq!(dock_config.font_size, 12);
                assert_eq!(dock_config.workspace_titles, true);
                assert_eq!(dock_config.active_fg_color, parse_color("#0000ff").unwrap());
                assert_eq!(dock_config.fg_color, parse_color("#ffffff").unwrap());
                assert_eq!(dock_config.active_bg_color, parse_color("#00ff00").unwrap());
                assert_eq!(dock_config.bg_color, parse_color("#000000").unwrap());
                assert_eq!(dock_config.border_color, parse_color("#333333").unwrap());
                assert_eq!(dock_config.border_width, 2);
                assert_eq!(dock_config.border_radius, 15);
                assert_eq!(dock_config.gap, 8);
                assert_eq!(
                    dock_config.margins,
                    NumMargins {
                        left: 4,
                        right: 4,
                        top: 4,
                        bottom: 4,
                    }
                );

                //window_button
                assert_eq!(dock_config.window_button.font_size, 12);
                assert_eq!(dock_config.window_button.title_width, 48.0);
                assert!(matches!(
                    dock_config.window_button.show_titles,
                    ShowTitles::Always
                ));
                assert_eq!(dock_config.window_button.wrap_titles, true);
                assert_eq!(
                    dock_config.window_button.icon_theme.as_ref().unwrap(),
                    "Adwaita"
                );
                assert_eq!(dock_config.window_button.icon_size, 24.0);
                assert_eq!(dock_config.window_button.line_height, 1.3);
                assert_eq!(dock_config.window_button.icon_opacity, 0.5);
                assert_eq!(dock_config.window_button.active_icon_opacity, 1.0);
                assert_eq!(
                    dock_config.window_button.icon_fallback.as_ref().unwrap(),
                    "?"
                );
                assert_eq!(
                    dock_config.window_button.fg_color,
                    parse_color("#ffffff").unwrap()
                );
                assert_eq!(
                    dock_config.window_button.active_fg_color,
                    parse_color("#0000ff").unwrap()
                );
                assert_eq!(
                    dock_config.window_button.bg_color,
                    parse_color("#000000").unwrap()
                );
                assert_eq!(
                    dock_config.window_button.active_bg_color,
                    parse_color("#00ff00").unwrap()
                );
                assert_eq!(
                    dock_config.window_button.border_color,
                    parse_color("#000000").unwrap()
                );
                assert_eq!(dock_config.window_button.border_width, 2.0);
                assert_eq!(dock_config.window_button.border_radius, 15.0);
                assert_eq!(dock_config.window_button.gap, 4.0);
                assert_eq!(
                    dock_config.window_button.margins,
                    NumMargins {
                        left: 2,
                        right: 2,
                        top: 2,
                        bottom: 2,
                    }
                );
            } else {
                panic!("Expected Dock widget");
            }
        } else {
            panic!("Expected WrapBox");
        }
    }
}
