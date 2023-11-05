pub struct PostProcessingPass {}

impl PostProcessingPass {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self) {}

    pub fn resize(&mut self) {}
}

impl Drop for PostProcessingPass {
    fn drop(&mut self) {
        todo!()
    }
}
