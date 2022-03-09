# Primitives

Primitive widgets are the most basic widgets that exist; they are generally unchanging and very unopinionated. Virtually every project that utilizes `agui` will use these as their most basic, most stable widgets.

## Layout

There are various widgets designed to abstract the layout to make their behavior clearer. It's recommended to use them whenever possible, so design and functionality are standardized both within your application and within all `agui` applications.

### Basics

- **Column**: A column stacks child widgets vertically, with an optional `spacing`.
- **Row**: A row lines up child widgets horizontally, with an optional `spacing`.
- **Padding**: Creates a widget with `margin`, effectively creating an internal padding in the widget.

### Spacing

The `Spacing` widget is another useful helper which can be used to create arbitrary distance between two widgets by utilizing its helper functions.

```rust,noplaypen
Spacing::none() // Creates the widget with null spacing.

Spacing::horizontal(Units) // Creates a widget with `Units` width.

Spacing::vertical(Units) // Creates a widget with `Units` height.
```

### What if I don't want to use it?

Well, then your life just got a lot more complicated. Widgets that don't wish to use `Drawable` must implement their own renderer in the integration they're using. See your integration's respective docs to see how to do this.

## Text

Just like `Drawable`, this is how you tell the renderer to draw Text in your interface. Its default functionality is slightly different from other widgets, so it's important to note here: by default, this widget will set its size to the width of the text as rendered. If you want it to take up less space, ensure you set its `sizing` field.