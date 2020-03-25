use std::collections::HashMap;

use crate::context::{
    graphics::{
        MeshBinding, SlicePipeline, SlicePlane, Transform4,
        TriangleListPipeline,
    },
    GraphicsContext,
};
use crate::physics::{
    calc_impulse, detect_collisions, Body, CollisionResponse,
};

pub struct Object {
    pub body: Body,
    pub mesh_binding: Option<MeshBinding>,
}

impl Object {
    pub fn compute(
        &self,
        graphics_ctx: &GraphicsContext,
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
                graphics_ctx,
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
        let mut mass_adjustments = HashMap::new();
        for i in 0..self.objects.len() {
            for j in i + 1..self.objects.len() {
                if let Some(manifold) = detect_collisions(
                    &self.objects[i].body,
                    &self.objects[j].body,
                ) {
                    *mass_adjustments.entry(i).or_insert(0) += 1;
                    *mass_adjustments.entry(j).or_insert(0) += 1;
                    collisions.push((i, j, manifold));
                }
            }
        }

        let mut impulses = Vec::new();
        let mut projections = Vec::new();
        for (i, j, collision) in collisions {
            let CollisionResponse {
                impulses: collision_impulses,
                projection,
            } = calc_impulse(
                collision,
                &self.objects[i].body,
                mass_adjustments[&i] as f32,
                &self.objects[j].body,
                mass_adjustments[&j] as f32,
            );
            for im in collision_impulses {
                impulses.push((i, -im.impulse, im.position));
                impulses.push((j, im.impulse, im.position));
            }
            projections.push((i, -projection));
            projections.push((j, projection));
        }

        for (i, impulse, world_contact) in impulses {
            self.objects[i].body.resolve_impulse(impulse, world_contact);
        }

        for (i, projection) in projections {
            self.objects[i].body.apply_projection(projection);
        }

        for object in self.objects.iter_mut() {
            object.body.step(dt);
        }
    }

    pub fn compute(
        &self,
        graphics_ctx: &GraphicsContext,
        pipeline: &SlicePipeline,
        encoder: &mut wgpu::CommandEncoder,
        slice_plane: &SlicePlane,
    ) {
        for i in self.objects.iter() {
            i.compute(graphics_ctx, pipeline, encoder, slice_plane);
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
