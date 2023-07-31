use agui_core::{
    callback::Callback,
    unit::Font,
    widget::{
        ContextWidgetStateMut, InheritedWidget, IntoWidget, StatefulBuildContext, StatefulWidget,
        Widget, WidgetState,
    },
};
use agui_macros::{build, InheritedWidget, StatefulWidget};

#[derive(StatefulWidget, Debug, Default)]
pub struct Fonts {
    pub fonts: im_rc::HashMap<String, Font>,

    pub child: Option<Widget>,
}

impl Fonts {
    pub fn new() -> Self {
        Self {
            fonts: im_rc::HashMap::default(),

            child: None,
        }
    }

    pub fn with_fonts(mut self, fonts: impl IntoIterator<Item = (String, Font)>) -> Self {
        self.fonts = im_rc::HashMap::from_iter(fonts);

        self
    }

    pub fn with_child(mut self, child: impl IntoWidget) -> Self {
        self.child = Some(child.into_widget());

        self
    }
}

impl StatefulWidget for Fonts {
    type State = FontsState;

    fn create_state(&self) -> Self::State {
        FontsState {
            initial_fonts: self.fonts.clone(),
            added_fonts: im_rc::HashMap::default(),
        }
    }
}

pub struct FontsState {
    initial_fonts: im_rc::HashMap<String, Font>,
    added_fonts: im_rc::HashMap<String, Font>,
}

impl WidgetState for FontsState {
    type Widget = Fonts;

    type Child = AvailableFonts;

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Self::Child {
        build! {
            AvailableFonts {
                available_fonts: self.added_fonts.clone().union(self.initial_fonts.clone()),

                add_font: ctx.callback(move |ctx, (name, font): (String, Font)| {
                    ctx.set_state(move |state| {
                        state.added_fonts.insert(name.to_string(), font);
                    });
                }),

                child: ctx.widget.child.clone(),
            }
        }
    }
}

#[derive(InheritedWidget, Default)]
pub struct AvailableFonts {
    available_fonts: im_rc::HashMap<String, Font>,

    // Allows us to modify the StatefulWidget state
    add_font: Callback<(String, Font)>,

    #[child]
    child: Option<Widget>,
}

impl InheritedWidget for AvailableFonts {}

impl AvailableFonts {
    pub fn get_font(&self, name: &str) -> Option<&Font> {
        self.available_fonts.get(name)
    }

    pub fn add_font(&self, name: String, font: Font) {
        self.add_font.call((name, font));
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_core::{
        manager::WidgetManager,
        unit::Font,
        widget::{BuildContext, ContextInheritedMut, WidgetBuild},
    };
    use agui_macros::StatelessWidget;

    use crate::{AvailableFonts, Fonts};

    #[derive(Default)]
    struct TestResult {
        retrieved_font: Option<Font>,
    }

    thread_local! {
        static TEST_HOOK: RefCell<TestResult> = RefCell::default();
    }

    #[derive(Default, StatelessWidget)]
    struct TestWidget;

    impl WidgetBuild for TestWidget {
        type Child = ();

        fn build(&self, ctx: &mut BuildContext<Self>) -> Self::Child {
            let available_fonts = ctx
                .depend_on_inherited_widget::<AvailableFonts>()
                .expect("failed to get available fonts");

            TEST_HOOK.with(|result| {
                result.borrow_mut().retrieved_font =
                    available_fonts.get_font("test font family").cloned();
            });
        }
    }

    #[test]
    fn can_retrieve_from_available_fonts() {
        let mut manager = WidgetManager::new();

        manager.set_root(
            Fonts::new()
                .with_fonts([(String::from("test font family"), Font::default())])
                .with_child(TestWidget),
        );

        manager.update();

        TEST_HOOK.with(|result| {
            assert_ne!(
                result.borrow().retrieved_font,
                None,
                "font was not retrieved"
            );
        })
    }
}
