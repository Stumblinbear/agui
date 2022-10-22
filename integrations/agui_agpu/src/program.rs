use std::ops::{Deref, DerefMut};

use agpu::GpuProgram;

use crate::ui::Agui;

pub struct AguiProgram {
    program: GpuProgram,

    ui: Agui,
}

impl AguiProgram {
    pub fn new(title: &str) -> Result<AguiProgram, agpu::BoxError> {
        Ok(Self::from(
            agpu::GpuProgram::builder(title)
                .with_framerate(f32::MAX)
                .build()?,
        ))
    }

    pub fn from(program: GpuProgram) -> Self {
        let ui = Agui::from_program(&program);

        Self { program, ui }
    }

    pub fn run(mut self) -> ! {
        self.program.run(move |event, program, _, _| {
            self.ui.handle_event(event, program);
        });
    }
}

impl Deref for AguiProgram {
    type Target = Agui;

    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl DerefMut for AguiProgram {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}
