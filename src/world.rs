use cgmath::Vector4;
use slotmap::{new_key_type, DenseSlotMap};
use std::collections::HashMap;

use crate::alg::Bivec4;
use crate::context::{
    graphics::{
        MeshBinding, ShadowPipeline, SlicePipeline, SlicePlane, Transform4,
        TriangleListPipeline,
    },
    GraphicsContext,
};
use crate::physics::{Body, CollisionConstraint, CollisionDetection, Material};
use crate::shapes;

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

    pub fn shadow_pass(
        &self,
        pipeline: &ShadowPipeline,
        render_pass: &mut wgpu::RenderPass,
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

    pub fn add_walls(
        &mut self,
        arena_size: f32,
        ctx: &GraphicsContext,
        slice_pipeline: &SlicePipeline,
    ) {
        self.objects.insert(shapes::create_floor(
            ctx,
            slice_pipeline,
            2.0 * arena_size,
            Material { restitution: 0.4 },
        ));

        // side walls
        self.objects.insert(shapes::create_wall(
            -arena_size * Vector4::unit_x(),
            Vector4::unit_x(),
            Material { restitution: 0.4 },
        ));
        self.objects.insert(shapes::create_wall(
            arena_size * Vector4::unit_x(),
            -Vector4::unit_x(),
            Material { restitution: 0.4 },
        ));
        self.objects.insert(shapes::create_wall(
            -arena_size * Vector4::unit_z(),
            Vector4::unit_z(),
            Material { restitution: 0.4 },
        ));
        self.objects.insert(shapes::create_wall(
            arena_size * Vector4::unit_z(),
            -Vector4::unit_z(),
            Material { restitution: 0.4 },
        ));
        self.objects.insert(shapes::create_wall(
            -arena_size * Vector4::unit_w(),
            Vector4::unit_w(),
            Material { restitution: 0.4 },
        ));
        self.objects.insert(shapes::create_wall(
            arena_size * Vector4::unit_w(),
            -Vector4::unit_w(),
            Material { restitution: 0.4 },
        ));
    }

    pub fn domino_track(
        &mut self,
        ctx: &GraphicsContext,
        slice_pipeline: &SlicePipeline,
    ) {
        self.objects.clear();
        self.add_walls(10.0, ctx, slice_pipeline);

        let mut domino = |x: f32, z: f32, w: f32, xz_deg: f32| {
            self.objects.insert(
                shapes::ShapeBuilder::new()
                    .cuboid(Vector4::new(1.0, 2.0, 0.3, 1.0))
                    .position(Vector4::new(x, 1.0, z, w))
                    .rotation(
                        Bivec4::new(
                            0.0,
                            xz_deg * std::f32::consts::PI / 360.0,
                            0.0,
                            0.0,
                            0.0,
                            0.0,
                        )
                        .exp(),
                    )
                    .build(ctx, slice_pipeline),
            )
        };

        domino(-4.0, 2.0, 0.0, 0.0);
        domino(-4.0, 1.0, 0.2, 0.0);
        domino(-4.0, 0.0, 0.4, 0.0);
        domino(-3.8, -1.0, 0.5, -15.0);
        domino(-3.5, -1.8, 0.6, -30.0);
        domino(-3.0, -2.4, 0.7, -45.0);
        domino(-2.5, -2.8, 0.8, -60.0);
        domino(-1.8, -3.1, 0.9, -75.0);
        domino(-1.0, -3.2, 1.0, -90.0);
        domino(0.0, -3.2, 1.2, -90.0);
        domino(1.0, -3.2, 1.4, -90.0);
        domino(2.0, -3.2, 1.2, -90.0);
        domino(2.8, -3.1, 1.0, -105.0);
        domino(3.5, -2.8, 0.9, -120.0);
        domino(4.0, -2.4, 0.8, -135.0);
        domino(4.5, -1.8, 0.7, -150.0);
        domino(4.8, -1.0, 0.6, -165.0);
        domino(5.0, 0.0, 0.5, -180.0);
        domino(5.0, 1.0, 0.3, -180.0);
        domino(5.0, 2.0, 0.1, -180.0);

        self.objects.insert(
            shapes::ShapeBuilder::new()
                .cuboid(Vector4::new(0.4, 3.0, 10.0, 5.0))
                .position(Vector4::new(0.0, 1.5, -2.0, -2.0))
                .color(Vector4::new(0.8, 0.8, 0.8, 1.0))
                .stationary(true)
                .build(ctx, slice_pipeline),
        );
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

        const SOLVER_ITERS: usize = 50;
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

    pub fn render(
        &self,
        pipeline: &TriangleListPipeline,
        render_pass: &mut wgpu::RenderPass,
    ) {
        for i in self.objects.values() {
            i.render(pipeline, render_pass);
        }
    }

    pub fn shadow_pass(
        &self,
        pipeline: &ShadowPipeline,
        render_pass: &mut wgpu::RenderPass,
    ) {
        for i in self.objects.values() {
            i.shadow_pass(pipeline, render_pass);
        }
    }
}
