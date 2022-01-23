# Fonts

`agui` comes with a built-in font system. In order to render text, you must begin by loading the font into `agui`; this is because the layout system must know how large text will be before rendering. While each integration may have their own methods for doing so, most of them should follow the same general convention. Using `agui_agpu` as an example, we just need to load the font file, or bytes:

```rust,noplaypen
# fn main() -> Result<(), agpu::BoxError> {
#     let program = agpu::GpuProgram::builder("agui fonts")
#         // The integration requires a few GPU features to be enabled
#         .with_gpu_features(
#                 Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
#                 | Features::VERTEX_WRITABLE_STORAGE,
#         )
#         .build()?;
# 
    // Create a UI with the default render passes
    let mut ui = UI::with_default(&program);

    // Import font bytes directly
    let font = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"));

    // Import a font file
    let font = ui.load_font_file("./fonts/DejaVuSans.ttf");
# }
```

The function returns a `FontId` which is used to reference the font in your UI. This can be stored however you like, and is generally used when creating `Text` widgets:

```rust,noplaypen
Text::is(font, 32.0, "Hello, world!".into())
```

## Supported Font Formats

We use `glyph_brush` to handle laying out fonts/glyphs, which itself utilizes `ab_glyph`. Therefore, we support any font format they do. At the time of writing, `agui` only supports loading TTF fonts.