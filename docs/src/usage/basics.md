# Basics

If you come from a background of Flutter or React/Vue, most of this should be familiar to you. However, I'll explain for those that have never heard of them.

User interfaces are inheritly complex and must be orchestrated properly to keep the visuals in line with the actual state of the program. Generally, this is a complex problem, and many issues can (and will) occur if you leave this "refreshing" up to the actual logic of your code. What a reactive UI library does is abstract your interface a bit to provide automatic updates to it whenever your state changes, ensuring your visuals and your state are always in sync.

However, with this ~magic~ comes additional considerations to your code. When state changes, all widgets listening to that state are updated in the UI tree. This means that you need to put some thought into limiting these rebuilds to as small of a piece of the tree as you can manage. While `agui` manages to reduce rebuilds where it can, it's not a magic bullet; we forgo tree-diffing for performance reasons, and to prevent problematic edge cases.

Hopefully that wasn't too much jargon for you. Just in case, here's a tl;dr: **UIs are complex, use `agui` to make them less of a pain to handle.**

## Glossary

- **Widget**: A user interface is built on Widgets, which can be anything from pure data, to managers, to elements drawn on screen. `agui` makes little distinction between them.
- **Layout**: `agui` leverages [morphorm](https://github.com/geom3trik/morphorm) for its layout system, which itself is modeled after the [subform layout system](https://subformapp.com/articles/why-not-flexbox/).
- **State**: At its core, `agui` is a state manager. It takes in your application state, and manages its lifecycle end-to-end, listening for changes and updating your widgets as necessary.
- **Global**: A global is state that exists as a singleton within your application. All widgets will read and write the same data.
- **Plugin**: A plugin is essentially a singleton widget that does not exist in the tree. They are often used to manage a global state that other widgets may listen to.
- **Computed Functions**: A function returns a value, only causing updates to the widget if the returned value changes.
- **Key**: A key is a way to instruct `agui` to cache a widget between rebuilds.