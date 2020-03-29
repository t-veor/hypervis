use std::collections::HashMap;

use crate::context::{
    graphics::{
        MeshBinding, SlicePipeline, SlicePlane, Transform4,
        TriangleListPipeline,
    },
    GraphicsContext,
};
use crate::physics::{detect_collisions, Body, CollisionConstraint};

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
                    if manifold.contacts.len() == 0 {
                        continue;
                    }
                    *mass_adjustments.entry(i).or_insert(0) += 1;
                    *mass_adjustments.entry(j).or_insert(0) += 1;
                    collisions.push((i, j, manifold));
                }
            }
        }

        let mut constraints = Vec::new();
        let mut projections = Vec::new();
        for (i, j, manifold) in collisions {
            // Do some linear projection to stop bodies from just sinking into
            // each other
            let slop_limit = 0.01f32;
            let slop_amount = 0.8f32;
            let projection = (manifold.depth - slop_limit).max(0.0)
                * slop_amount
                * manifold.normal;

            projections.push((i, -projection));
            projections.push((j, projection));
            constraints.push((
                i,
                j,
                CollisionConstraint::new(
                    manifold,
                    &self.objects[i].body,
                    mass_adjustments[&i] as f32,
                    &self.objects[j].body,
                    mass_adjustments[&j] as f32,
                ),
            ));
        }

        const SOLVER_ITERS: usize = 20;
        for _ in 0..SOLVER_ITERS {
            for (i, j, constraint) in constraints.iter_mut() {
                assert!(i < j);

                let (head, tail) = self.objects.split_at_mut(*j);
                constraint.solve(&mut head[*i].body, &mut tail[0].body);
            }
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
