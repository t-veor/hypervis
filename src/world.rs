use slotmap::{new_key_type, DenseSlotMap};
use std::collections::HashMap;

use crate::context::{
    graphics::{
        MeshBinding, ShadowPipeline, SlicePipeline, SlicePlane, Transform4,
        TriangleListPipeline,
    },
    GraphicsContext,
};
use crate::physics::{Body, CollisionConstraint, CollisionDetection};

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

    pub fn render<'a: 'c, 'b, 'c>(
        &'a self,
        pipeline: &'a TriangleListPipeline,
        render_pass: &'b mut wgpu::RenderPass<'c>,
    ) {
        if let Some(mesh_binding) = &self.mesh_binding {
            pipeline.render(render_pass, mesh_binding);
        }
    }

    pub fn shadow_pass<'a: 'c, 'b, 'c>(
        &'a self,
        pipeline: &'a ShadowPipeline,
        render_pass: &'b mut wgpu::RenderPass<'c>,
    ) {
        if let Some(mesh_binding) = &self.mesh_binding {
            pipeline.render(render_pass, mesh_binding);
        }
    }
}

new_key_type! { pub struct ObjectKey; }

pub struct World {
    pub objects: DenseSlotMap<ObjectKey, Object>,
    pub collision: CollisionDetection,
}

fn slotmap_get_mut2<K, V>(
    map: &mut DenseSlotMap<K, V>,
    i: K,
    j: K,
) -> (&mut V, &mut V)
where
    K: slotmap::Key + std::cmp::Eq,
{
    assert!(i != j);

    unsafe {
        let a = std::mem::transmute::<_, _>(map.get_mut(i).unwrap());
        let b = std::mem::transmute::<_, _>(map.get_mut(j).unwrap());

        return (a, b);
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            objects: DenseSlotMap::with_key(),
            collision: CollisionDetection::new(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        let mut collisions = Vec::new();
        let mut mass_adjustments = HashMap::new();

        let object_keys: Vec<_> = self.objects.keys().collect();

        for i in 0..object_keys.len() {
            for j in i + 1..object_keys.len() {
                let ka = object_keys[i];
                let kb = object_keys[j];
                let a = &self.objects[ka];
                let b = &self.objects[kb];

                if let Some(manifold) =
                    self.collision.detect_collisions((ka, kb), &a.body, &b.body)
                {
                    if manifold.contacts.len() == 0 {
                        continue;
                    }
                    *mass_adjustments.entry(ka).or_insert(0) += 1;
                    *mass_adjustments.entry(kb).or_insert(0) += 1;
                    collisions.push((ka, kb, manifold));
                }
            }
        }

        let mut constraints = Vec::new();
        for (i, j, manifold) in collisions {
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
                let (a, b) = slotmap_get_mut2(&mut self.objects, *i, *j);
                constraint.solve(&mut a.body, &mut b.body);
            }
        }

        for object in self.objects.values_mut() {
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
        for i in self.objects.values() {
            i.compute(graphics_ctx, pipeline, encoder, slice_plane);
        }
    }

    pub fn render<'a: 'c, 'b, 'c>(
        &'a self,
        pipeline: &'a TriangleListPipeline,
        render_pass: &'b mut wgpu::RenderPass<'c>,
    ) {
        for i in self.objects.values() {
            i.render(pipeline, render_pass);
        }
    }

    pub fn shadow_pass<'a: 'c, 'b, 'c>(
        &'a self,
        pipeline: &'a ShadowPipeline,
        render_pass: &'b mut wgpu::RenderPass<'c>,
    ) {
        for i in self.objects.values() {
            i.shadow_pass(pipeline, render_pass);
        }
    }
}
