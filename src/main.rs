//! Dreg-based File Manager



use dreg::prelude::*;



fn main() -> Result<()> {
    CrosstermPlatform::new()?
        .run(FileManager {
            should_exit: false,
        })
}



pub struct FileManager {
    should_exit: bool,
}

impl Program for FileManager {
    fn update(&mut self, _frame: Frame) {
    }

    fn on_input(&mut self, input: Input) {
        match input {
            Input::KeyDown(Scancode::Q) => {
                self.should_exit = true;
            }
            _ => {}
        }
    }

    fn on_platform_request(&mut self, _request: &str) -> Option<&str> { None }

    fn should_exit(&self) -> bool { self.should_exit }
}

