# Fonts

`agui` comes with a built-in font system. In order to render text, you must begin by loading the font into `agui`; this is because the layout system must know how large text will be before rendering. While each integration may have their own methods for doing so, most of them should follow the same general convention. Using `agui_wgpu` as an example, we just need to load the font file, or bytes:

```rust,noplaypen
# fn main() {
#     let mut ui = UIProgram::new("agui hello world")?;
#
#
    // Import font bytes directly
    let font = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"));

    // Import a font file
    let font = ui.load_font_file("./fonts/DejaVuSans.ttf");
# }
```

The function returns a `Font` which is used to reference the font in your UI. This can be stored however you like, and is generally used when creating `Text` widgets:

```rust,noplaypen
Text {
    font: deja_vu.styled().size(32.0),
    text: "Hello, world!"
}
```

## Supported Font Formats

We use `glyph_brush` to handle laying out fonts/glyphs, which itself utilizes `ab_glyph`. Therefore, we support any font format they do. At the time of writing, `agui` only supports loading TTF fonts.
