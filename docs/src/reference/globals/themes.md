# Themes

Having a standardized Theme system is necessary for an easy-to-use UI library. However, we don't use a single struct for styles, as this is wasteful and would not cover every use case. Instead, each widget should create a struct with style information, that derives the traits: `Default + Send + Sync`.

## Usage

Lets go over how the `Button` widget handles its styling, as an example:

```rust,noplaypen
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub normal: Color,
    pub disabled: Color,
    pub hover: Color,
    pub pressed: Color,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal: Color::White,
            disabled: Color::LightGray,
            hover: Color::LightGray,
            pressed: Color::DarkGray,
        }
    }
}
```

This houses all of the fields that the `Button` widget uses to determine how it will render. When it actually wants to utilize that style, we use the `StyleExt` extension trait.

```rust,noplaypen
use agui::widgets::state::theme::StyleExt;

# #[functional_widget]
fn button(ctx: &BuildContext, style: Option<ButtonStyle>, child: WidgetRef) -> BuildResult {
    // `resolve` will perform the following steps to get the style:
    //   1. If the style is `Some`, return it
    //   2. Check for a widget that's providing a Theme, and get_or_default from that
    //   3. Check global state for a Theme, and get_or_default from that
    //   4. Use the Default style
    let style: ButtonStyle = style.resolve(ctx);

    BuildResult::None
}
```

Notice that the `style` field is an `Option`. The `StyleExt` trait also supports this type, making it simple to allow style overrides without additional checks.