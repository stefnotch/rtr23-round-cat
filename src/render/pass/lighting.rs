pub struct LightingPass {
    // render_pass: vk::RenderPass,
    // pipeline: vk::Pipeline,
}

impl LightingPass {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self) {}

    pub fn resize(&mut self) {}
}

impl Drop for LightingPass {
    fn drop(&mut self) {}
}
