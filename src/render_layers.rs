use bevy::{camera::visibility::RenderLayers, prelude::*};

pub fn plugin(app: &mut App) {
    app.init_resource::<CameraRenderLayers>();
}

#[derive(Resource, Default)]
pub struct CameraRenderLayers(usize);
impl CameraRenderLayers {
    const START: usize = 64;

    pub fn next(&mut self) -> RenderLayers {
        let render_layers = RenderLayers::from_layers(&[0, Self::START + self.0]);
        self.0 += 1;
        render_layers
    }
}
