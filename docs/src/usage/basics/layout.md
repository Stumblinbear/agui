# Layout

`agui` leverages [morphorm](https://github.com/geom3trik/morphorm) for its layout system, which itself is modeled after the [subform layout system](https://subformapp.com/articles/why-not-flexbox/). While we go into some detail here, it's recommended to do some research into those as well until this page is fleshed out a bit more.

## Why subform? Why not flexbox?

Because, frankly, flexbox is confusing. We need something simpler, that doesn't interact with itself in invisible ways or bring in new concepts such as `align-items`, `justify-content`, or `align-self`. According to Subform themselves, the tl;dr is:

> - All elements have a horizontal and vertical axis, each of which consists of space before, size, and space after.
> - Elements either control their own position (“self-directed”, akin to CSS absolute positioning) or are positioned by their parent (parent-directed).
> - Parents can position their parent-directed children in a vertical stack, horizontal stack, or grid.
> - The same units—pixels, percentages (of the parent size), and stretch (akin to flex, proportionally dividing up available space)—are used everywhere, with minimum and maximum constraints as part of the unit.

At its most basic level, your layouts are just rows and columns of widgets, each of which may contain more rows and columns of widgets. Beyond that, sizes of your widgets can be pixels, a percentage of the parent, or stretch to fill. That's it. Stupid simple.

## How do I use it?

There are some primitive widgets that make the layout system easier to grok. `Column`, `Row`, `Grid`, `Padding`, and `Spacing`, each of which simply abstract out the layout. While you could use `ctx.set_layout` for each widget yourself, it's recommended to use these widgets instead, as it makes your widgets simpler and more reusable.
