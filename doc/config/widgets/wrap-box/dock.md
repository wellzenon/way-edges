# Dock

```kdl
item "dock" {
    index 0 0
    grid-align "center-center"
    font-family "mono"
    font-size 26
    workspace-titles false
    fg-color "#ffffff55"
    bg-color "#00000000"
    active-fg-color "#ffffffaa"
    active-bg-color "#00000000"
    border-color "#00000000"
    active-border-color "#00000000"
    border-width 0
    border-radius 10
    gap 20
    separator-width 1
    separator-radius 0
    separator-margin 6
    separator-color "#444444"
    margins {
        left 3
        right 3
        top 3
        bottom 3
    }
    window-button {
        font-family "mono"
        font-size 16
        line-height 1.4
        title-width 60
        show-titles "focused"
        wrap-titles false
        icon-theme "hicolor"
        icon-size 32
        icon-fallback ""
        icon-opacity 0.5
        active-icon-opacity 1
        fg-color "#88888888"
        active-fg-color "#dddddd"
        bg-color "#00000000"
        active-bg-color "#ffffff10"
        border-color "#00000000"
        active-border-color "#bbbbbb"
        border-width 1
        border-radius 5
        gap 5
        margins {
            left 5
            right 5
            top 5
            bottom 5
        }
    }
}
```

| Name                | Description                                                                                                                            |
| ------------------  | -------------------------------------------------------------------------------------------------------------------------------------- |
| type                | const `dock`                                                                                                                           |
| font-family         | font family                                                                                                                            |
| grid-align          | 9 positions: center-left, center-right, top-left, top-right, bottom-left, bottom-right, left-top, left-bottom, right-top, right-bottom |
| font-size           | int |
| workspace-titles    | bool |
| fg-color            | color |
| active-fg-color     | color |
| bg-color            | color |
| active-bg-color     | color |
| border-color        | color |
| active-border-color    | color |
| border-width        | int |
| border-radius       | int |
| gap                 | int |
| separator-width        | int |
| separator-radius        | int |
| separator-margin        | int |
| separator-color        | color |
| margins             |  |

## window-button

| Name                | Description |
| ------------------- | ----------- |
| font-family         | font family                                                                                                                            |
| font-size           | int |
| line-height         | float |
| title-width         | int |
| show-titles         | 4 options: always, never, focused, only (only option won't show icons)|
| wrap-titles         | bool |
| icon-theme          | string |
| icon-size           | int |
| icon-fallback       | path to image file or 1 char or null to fallback to the title first letter |
| icon-opacity        | float: 0.0 to 1.0 |
| active-icon-opacity | float: 0.0 to 1.0 |
| fg-color            | color |
| active-fg-color     | color |
| bg-color            | color |
| active-bg-color     | color |
| active-border-color        | color |
| border-width        | int |
| border-radius       | int |
| gap                 | int |
| margins             | |

