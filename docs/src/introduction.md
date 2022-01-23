> **Preface**
> 
> This book (or guide, whatever you want to call it) does not always take itself seriously (I mean, the library is called `agui`, what did you expect?). This is largely because I (the writer) personally find the Rust ecosystem's documentation dreadfully dull at the best of times. Now, for core libraries, I understand it; I even appreciate it! It's important that core functionality is explained well, and explained in great detail, leaving no room for misinterpretation (a comma mustn't end up being the debate of the century).
> 
> That said: I don't believe this library deserves to be treated in such a robotic manner. When building UIs, we no longer are just code monkeys, tasked with building things users will never directly interact with, hiding flaws in the deepest foundation only we can see. In here, we draw, we paint, we render. We, in effect, become artists ourselves; and a robotic artist is a starving artist.
> 
> I've put personality in this book (guide) to reflect that reality. Why can't programming be fun?

# Introduction

**Agui** is a reactive UI library, built in pure Rust, made for pure Rust. It's inspired by Flutter and other reactive UI libraries, and almost more importantly: it's renderer-agnostic.  There are a few basic concepts to learn, but if you come from a background of Flutter, React, Vue, or other similar libraries, you'll feel right at home. However, I'll explain for those that have never heard of them.

User interfaces are inheritly complex and must be orchestrated properly to keep the visuals in line with the actual state of the program. Generally, this is a complex problem, and many issues can (and will) occur if you leave this "refreshing" up to the actual logic of your code. What a reactive UI library does is abstract your interface a bit to provide automatic updates to it whenever your state changes, ensuring your visuals and your state are always in sync.

However, with this ~magic~ comes additional considerations to your code. When state changes, all widgets listening to that state are updated in the UI tree. This means that you need to put some thought into limiting these rebuilds to as small of a piece of the tree as you can manage. While `agui` manages to reduce rebuilds where it can, it's not a magic bullet; we forgo tree-diffing for performance reasons, and to prevent problematic edge cases.

Hopefully that wasn't too much jargon for you. Just in case, here's a tl;dr: **UIs are complex, use `agui` to make them less of a pain to handle.**

## Glossary

- **Widget**: A user interface is built on Widgets, which can be anything from pure data, to managers, to elements drawn on screen. `agui` makes little distinction between them.
- **Layout**: `agui` leverages [morphorm](https://github.com/geom3trik/morphorm) for its layout system, which itself is modeled after the [subform layout system](https://subformapp.com/articles/why-not-flexbox/).
- **State**: At its core, `agui` is a state manager. It takes in your application's variables, and manages their lifecycle end-to-end, listening for changes and updating your widgets as necessary.
- **Global**: A global is state that exists as a singleton within your application. All widgets will read and write the same data.
- **Plugin**: A plugin is essentially a singleton widget that does not exist in the tree. They are often used to manage one (or more) globals that other widgets may listen to.
- **Computed Values**: A function that returns a value, only causing updates to the widget if the returned value changes.
- **Key**: A key is a way to instruct `agui` to cache a widget between rebuilds.

## Stability Warning

While the core of `Agui` is mostly stable, it is still very much in its infancy. The API may change, and optimizations still need to be done, especially when it comes to talking to the render integrations, and the integrations themselves. We are still finding our way, and that will take time. That said: it works. If you need a feature that doesn't exist, feel free to contribute or make a plugin!

## Contributing

Agui is free and open source. You can find the source code on [GitHub](https://github.com/stumblinbear/agui), issues can be posted on the [GitHub issue tracker](https://github.com/stumblinbear/agui/issues), with feature requests and questions directed to [Github discussions](https://github.com/stumblinbear/agui/discussions).