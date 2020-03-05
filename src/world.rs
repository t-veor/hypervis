use crate::context::graphics::MeshBinding;
use crate::physics::{collide, Body};

pub struct Object {
    pub body: Body,
    pub mesh_binding: MeshBinding,
}

pub struct World {
    objects: Vec<Object>,
}

impl World {
    pub fn update(&mut self, dt: f32) {
        let mut collisions = Vec::new();

        for i in 0..self.objects.len() {
            for j in i..self.objects.len() {
                if let Some(impulse) =
                    collide(&self.objects[i].body, &self.objects[j].body)
                {
                    collisions.push((i, j, impulse))
                }
            }
        }

        for (i, j, impulse) in collisions {
            self.objects[i].body.apply_impulse(-impulse);
            self.objects[j].body.apply_impulse(impulse);
        }

        for object in self.objects.iter_mut() {
            object.body.step(dt);
        }
    }
}
