use std::ops::{Deref, DerefMut};

use agpu::GpuProgram;

use crate::ui::UI;

pub struct UIProgram {
    program: GpuProgram,

    ui: UI<'static>,
}

impl UIProgram {
    pub fn new(title: &str) -> Result<UIProgram, agpu::BoxError> {
        Ok(Self::from(
            agpu::GpuProgram::builder(title)
                .with_framerate(f32::MAX)
                .build()?,
        ))
    }

    pub fn from(program: GpuProgram) -> Self {
        let ui = UI::from_program(&program);

        Self { program, ui }
    }

    pub fn run(mut self) -> ! {
        self.program.run(move |event, program, _, _| {
            self.ui.handle_event(event, program);
        });
    }
}

impl Deref for UIProgram {
    type Target = UI<'static>;

    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl DerefMut for UIProgram {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}
