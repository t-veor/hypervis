use cgmath::{InnerSpace, Vector4, Zero};

use crate::alg::{Bivec4, Rotor4};
use crate::context::{graphics::SlicePipeline, GraphicsContext};
use crate::mesh::{Mesh, TetrahedronMesh};
use crate::physics::{Body, Collider, Material, MomentInertia, Velocity};
use crate::world::Object;

pub enum RegularSolid {
    FiveCell,
    EightCell,
    SixteenCell,
    TwentyFourCell,
    OneTwentyCell,
    SixHundredCell,
}

enum ShapeSpec {
    RegularSolid { ty: RegularSolid },
    Sphere { radius: f32 },
    Cuboid { dimensions: Vector4<f32> },
}

pub fn create_floor(
    ctx: &GraphicsContext,
    slice_pipeline: &SlicePipeline,
    size: f32,
    material: Material,
) -> Object {
    let floor_mesh = crate::mesh4::floor(size);
    let floor_mesh_binding = slice_pipeline.create_mesh_binding(
        &ctx,
        &floor_mesh.vertices,
        &floor_mesh.indices,
    );
    Object {
        body: Body {
            mass: 0.0,
            moment_inertia: MomentInertia::Isotropic(0.0),
            material,
            stationary: true,
            pos: Vector4::zero(),
            rotation: Rotor4::identity(),
            vel: Velocity::zero(),
            collider: Collider::HalfSpace {
                normal: Vector4::unit_y(),
            },
        },
        mesh_binding: Some(floor_mesh_binding),
    }
}

pub fn create_wall(
    position: Vector4<f32>,
    normal: Vector4<f32>,
    material: Material,
) -> Object {
    Object {
        body: Body {
            mass: 0.0,
            moment_inertia: MomentInertia::Isotropic(0.0),
            material,
            stationary: true,
            pos: position,
            rotation: Rotor4::identity(),
            vel: Velocity::zero(),
            collider: Collider::HalfSpace {
                normal: normal.normalize(),
            },
        },
        mesh_binding: None,
    }
}

pub struct ShapeBuilder {
    spec: ShapeSpec,
    position: Vector4<f32>,
    rotation: Rotor4,
    velocity: Velocity,
    stationary: bool,
    mass: f32,
    material: Material,
    color: Option<Vector4<f32>>,
}

impl ShapeBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn regular_solid(mut self, ty: RegularSolid) -> Self {
        self.spec = ShapeSpec::RegularSolid { ty };
        self
    }

    pub fn sphere(mut self, radius: f32) -> Self {
        self.spec = ShapeSpec::Sphere { radius };
        self
    }

    pub fn cuboid(mut self, dimensions: Vector4<f32>) -> Self {
        self.spec = ShapeSpec::Cuboid { dimensions };
        self
    }

    pub fn position(mut self, position: Vector4<f32>) -> Self {
        self.position = position;
        self
    }

    pub fn rotation(mut self, rotation: Rotor4) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn velocity(mut self, velocity: Vector4<f32>) -> Self {
        self.velocity.linear = velocity;
        self
    }

    pub fn angular_velocity(mut self, angular_velocity: Bivec4) -> Self {
        self.velocity.angular = angular_velocity;
        self
    }

    pub fn stationary(mut self, stationary: bool) -> Self {
        self.stationary = stationary;
        self
    }

    pub fn mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }

    pub fn material(mut self, material: Material) -> Self {
        self.material = material;
        self
    }

    pub fn color(mut self, color: Vector4<f32>) -> Self {
        self.color = Some(color);
        self
    }

    pub fn random_color(mut self) -> Self {
        self.color = None;
        self
    }

    pub fn build(
        self,
        ctx: &GraphicsContext,
        slice_pipeline: &SlicePipeline,
    ) -> Object {
        use hsl::HSL;

        let (mesh_binding, collider, moment_inertia) = match self.spec {
            ShapeSpec::RegularSolid { ty } => {
                let schlafli_symbol = match ty {
                    RegularSolid::FiveCell => &[3, 3, 3],
                    RegularSolid::EightCell => &[4, 3, 3],
                    RegularSolid::SixteenCell => &[3, 3, 4],
                    RegularSolid::TwentyFourCell => &[3, 4, 3],
                    RegularSolid::OneTwentyCell => &[5, 3, 3],
                    RegularSolid::SixHundredCell => &[3, 3, 5],
                };

                let mesh = Mesh::from_schlafli_symbol(schlafli_symbol);
                let color = self.color;
                let tetrahedralized_mesh =
                    TetrahedronMesh::from_mesh(&mesh, |normal| {
                        color.unwrap_or_else(|| {
                            let (r, g, b) = HSL {
                                h: 180.0
                                    * (normal.z as f64
                                        + rand::random::<f64>() * 5.0
                                        - 2.5)
                                    % 360.0
                                    + 360.0,
                                s: 0.8,
                                l: 0.5 + rand::random::<f64>() * 0.1,
                            }
                            .to_rgb();
                            Vector4::new(
                                r as f32 / 255.0,
                                g as f32 / 255.0,
                                b as f32 / 255.0,
                                1.0,
                            )
                        })
                    });
                let mesh_binding = slice_pipeline.create_mesh_binding(
                    &ctx,
                    &tetrahedralized_mesh.vertices,
                    &tetrahedralized_mesh.indices,
                );
                (
                    mesh_binding,
                    Collider::Mesh { mesh },
                    MomentInertia::Isotropic(self.mass / 6.0), // TODO!
                )
            }
            ShapeSpec::Sphere { radius } => {
                let mesh = Mesh::from_schlafli_symbol(&[3, 3, 5]);
                let color = self.color.unwrap_or_else(|| {
                    let (r, g, b) = HSL {
                        h: 360.0 * rand::random::<f64>(),
                        s: 1.0,
                        l: 0.5 + rand::random::<f64>() * 0.1,
                    }
                    .to_rgb();
                    Vector4::new(
                        r as f32 / 255.0,
                        g as f32 / 255.0,
                        b as f32 / 255.0,
                        1.0,
                    )
                });
                let tetrahedralized_mesh =
                    TetrahedronMesh::from_mesh(&mesh, |_| color)
                        .make_geodesic(4, radius);
                let mesh_binding = slice_pipeline.create_mesh_binding(
                    &ctx,
                    &tetrahedralized_mesh.vertices,
                    &tetrahedralized_mesh.indices,
                );
                (
                    mesh_binding,
                    Collider::Sphere { radius },
                    MomentInertia::Isotropic(radius * radius * self.mass / 2.0),
                )
            }
            ShapeSpec::Cuboid { dimensions } => {
                let mut mesh = Mesh::from_schlafli_symbol(&[4, 3, 3]);
                mesh.vertices.iter_mut().for_each(|v| {
                    *v = Vector4::new(
                        v.x * dimensions.x,
                        v.y * dimensions.y,
                        v.z * dimensions.z,
                        v.w * dimensions.w,
                    );
                });
                mesh.radius *= dimensions.magnitude();

                let color = self.color;
                let tetrahedralized_mesh =
                    TetrahedronMesh::from_mesh(&mesh, |normal| {
                        color.unwrap_or_else(|| {
                            let (r, g, b) = HSL {
                                h: 180.0
                                    * (normal.z as f64
                                        + rand::random::<f64>() * 5.0
                                        - 2.5)
                                    % 360.0
                                    + 360.0,
                                s: 0.8,
                                l: 0.5 + rand::random::<f64>() * 0.1,
                            }
                            .to_rgb();
                            Vector4::new(
                                r as f32 / 255.0,
                                g as f32 / 255.0,
                                b as f32 / 255.0,
                                1.0,
                            )
                        })
                    });
                let mesh_binding = slice_pipeline.create_mesh_binding(
                    &ctx,
                    &tetrahedralized_mesh.vertices,
                    &tetrahedralized_mesh.indices,
                );

                let (x_2, y_2, z_2, w_2) = (
                    dimensions.x.powi(2),
                    dimensions.y.powi(2),
                    dimensions.z.powi(2),
                    dimensions.w.powi(2),
                );
                let moment_inertia = MomentInertia::Principal(
                    self.mass / 12.0
                        * Bivec4::new(
                            x_2 + y_2,
                            x_2 + z_2,
                            x_2 + w_2,
                            y_2 + z_2,
                            y_2 + w_2,
                            z_2 + w_2,
                        ),
                );

                (mesh_binding, Collider::Mesh { mesh }, moment_inertia)
            }
        };

        Object {
            body: Body {
                mass: self.mass,
                moment_inertia,
                material: self.material,
                stationary: self.stationary,
                pos: self.position,
                rotation: self.rotation,
                vel: self.velocity,
                collider,
            },
            mesh_binding: Some(mesh_binding),
        }
    }
}

impl Default for ShapeBuilder {
    fn default() -> Self {
        Self {
            spec: ShapeSpec::RegularSolid {
                ty: RegularSolid::FiveCell,
            },
            position: Vector4::unit_y() * 5.0,
            rotation: Rotor4::identity(),
            velocity: Velocity::zero(),
            stationary: false,
            mass: 1.0,
            material: Material { restitution: 0.2 },
            color: None,
        }
    }
}
