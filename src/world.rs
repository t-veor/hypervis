use crate::context::{
    graphics::{
        MeshBinding, SlicePipeline, SlicePlane, Transform4,
        TriangleListPipeline,
    },
    Ctx,
};
use crate::physics::{collide, Body};

pub struct Object {
    pub body: Body,
    pub mesh_binding: Option<MeshBinding>,
}

impl Object {
    pub fn compute(
        &self,
        ctx: &Ctx,
        pipeline: &SlicePipeline,
        encoder: &mut wgpu::CommandEncoder,
        slice_plane: &SlicePlane,
    ) {
        if let Some(mesh_binding) = &self.mesh_binding {
            let transform = Transform4 {
                displacement: self.body.pos,
                transform: self.body.rotation.to_matrix(),
            };
            pipeline.render_mesh(
                &ctx.graphics_ctx,
                encoder,
                slice_plane,
                &transform,
                mesh_binding,
            );
        }
    }

    pub fn render(
        &self,
        pipeline: &TriangleListPipeline,
        render_pass: &mut wgpu::RenderPass,
    ) {
        if let Some(mesh_binding) = &self.mesh_binding {
            pipeline.render(render_pass, mesh_binding);
        }
    }
}

pub struct World {
    pub objects: Vec<Object>,
}

impl World {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        let mut collisions = Vec::new();

        for i in 0..self.objects.len() {
            for j in i + 1..self.objects.len() {
                collide(&self.objects[i].body, &self.objects[j].body)
                    .drain(0..)
                    .for_each(|collision| collisions.push((i, j, collision)))
            }
        }

        for (i, j, collision) in collisions {
            self.objects[i].body.resolve_collision(&collision, true);
            self.objects[j].body.resolve_collision(&collision, false);
        }

        for object in self.objects.iter_mut() {
            object.body.step(dt);
        }
    }

    pub fn compute(
        &self,
        ctx: &Ctx,
        pipeline: &SlicePipeline,
        encoder: &mut wgpu::CommandEncoder,
        slice_plane: &SlicePlane,
    ) {
        for i in self.objects.iter() {
            i.compute(ctx, pipeline, encoder, slice_plane);
        }
    }

    pub fn render(
        &self,
        pipeline: &TriangleListPipeline,
        render_pass: &mut wgpu::RenderPass,
    ) {
        for i in self.objects.iter() {
            i.render(pipeline, render_pass);
        }
    }
}
