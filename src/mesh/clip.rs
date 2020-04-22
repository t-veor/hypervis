// Adapted from https://www.geometrictools.com/Documentation/ClipMesh.pdf

use super::{Cell, Mesh};
use cgmath::{InnerSpace, Vector4};
use smallvec::{smallvec, SmallVec};
use std::collections::HashMap;

use crate::util::EPSILON;

#[derive(Debug)]
struct ClipVertex {
    point: Vector4<f32>,
    distance: f32,
    // temporary variable used for getting the open polyline. Can be initialised
    // to anything you want - it'll be reset before it's used
    occurs: i32,
    visible: bool,
}

#[derive(Debug)]
struct ClipEdge {
    hd_vertex: usize,
    tl_vertex: usize,
    faces: SmallVec<[usize; 8]>,
    visible: bool,
}

#[derive(Debug)]
struct ClipFace {
    edges: SmallVec<[usize; 8]>,
    visible: bool,
}

pub struct ClipMesh {
    vertices: Vec<ClipVertex>,
    edges: Vec<ClipEdge>,
    faces: Vec<ClipFace>,
}

enum ProcessVertexResult {
    NoneClipped,
    AllClipped,
    PartiallyClipped,
}

impl ClipMesh {
    pub fn from_cell(mesh: &Mesh, cell_idx: usize) -> Self {
        ClipMeshBuilder::new(mesh, cell_idx).build()
    }

    pub fn clip_by(&mut self, clip_normal: Vector4<f32>, clip_distance: f32) {
        match self.process_vertices(clip_normal, clip_distance) {
            ProcessVertexResult::NoneClipped => return,
            ProcessVertexResult::AllClipped => {
                for edge in self.edges.iter_mut() {
                    edge.visible = false;
                }

                for face in self.faces.iter_mut() {
                    face.visible = false;
                }

                return;
            }
            ProcessVertexResult::PartiallyClipped => (),
        };

        self.process_edges();
        self.process_faces();
    }

    pub fn to_vertices(self) -> Vec<Vector4<f32>> {
        self.vertices
            .into_iter()
            .filter_map(|v| if v.visible { Some(v.point) } else { None })
            .collect()
    }

    fn process_vertices(
        &mut self,
        clip_normal: Vector4<f32>,
        clip_distance: f32,
    ) -> ProcessVertexResult {
        let mut positive = 0;
        let mut negative = 0;

        for vertex in self.vertices.iter_mut() {
            if !vertex.visible {
                continue;
            }

            vertex.distance = clip_normal.dot(vertex.point) - clip_distance;

            if vertex.distance >= EPSILON {
                positive += 1;
            } else if vertex.distance < -EPSILON {
                negative += 1;
                vertex.visible = false;
            } else {
                vertex.distance = 0.0;
            }
        }

        if negative == 0 {
            return ProcessVertexResult::NoneClipped;
        } else if positive == 0 {
            return ProcessVertexResult::AllClipped;
        } else {
            return ProcessVertexResult::PartiallyClipped;
        }
    }

    fn process_edges(&mut self) {
        for (edge_idx, edge) in self.edges.iter_mut().enumerate() {
            if !edge.visible {
                continue;
            }

            let d0 = self.vertices[edge.hd_vertex].distance;
            let d1 = self.vertices[edge.tl_vertex].distance;

            if d0 <= 0.0 && d1 <= 0.0 {
                // edge is culled, remove edge from faces sharing it
                for face in self.faces.iter_mut() {
                    for i in 0..face.edges.len() {
                        if face.edges[i] == edge_idx {
                            face.edges.remove(i);
                            break;
                        }
                    }
                    if face.edges.len() == 0 {
                        face.visible = false;
                    }
                }
                edge.visible = false;
            } else if d0 >= 0.0 && d1 >= 0.0 {
                // edge is on nonnegative side, faces retain the edge
            } else {
                // edge is split by plane
                // If the old edge is <v0, v1> and I is the intersection
                // point, the new edge os <v0, I> when d0 > 0 or <I, v1>
                // when d1 > 0.

                // d0 and d1 are at least 2 EPSILONS apart here so this is
                // fine
                let t = d0 / (d0 - d1);
                let intersection = (1.0 - t)
                    * self.vertices[edge.hd_vertex].point
                    + t * self.vertices[edge.tl_vertex].point;

                let fresh_idx = self.vertices.len();
                self.vertices.push(ClipVertex {
                    point: intersection,
                    distance: 0.0,
                    occurs: 0,
                    visible: true,
                });

                if d0 > 0.0 {
                    edge.tl_vertex = fresh_idx;
                } else {
                    edge.hd_vertex = fresh_idx;
                }
            }
        }
    }

    fn process_faces(&mut self) {
        // If this is called, then the mesh straddles the plane. A new convex
        // polygonal face will be generated. Add it now and insert edges when
        // they are visited.
        let close_face = ClipFace {
            edges: SmallVec::new(),
            visible: true,
        };
        let close_face_idx = self.faces.len();
        self.faces.push(close_face);

        for face_idx in 0..self.faces.len() {
            if !self.faces[face_idx].visible {
                continue;
            }

            if let Some((start, end)) = self.get_open_polyline(face_idx) {
                let close_edge = ClipEdge {
                    hd_vertex: start,
                    tl_vertex: end,
                    faces: smallvec![face_idx, close_face_idx],
                    visible: true,
                };
                let fresh_edge_idx = self.edges.len();
                self.edges.push(close_edge);
                self.faces[face_idx].edges.push(fresh_edge_idx);

                self.faces[close_face_idx].edges.push(fresh_edge_idx);
            }
        }
    }

    fn get_open_polyline(&mut self, face_idx: usize) -> Option<(usize, usize)> {
        let face = &self.faces[face_idx];

        for edge_idx in face.edges.iter() {
            let edge = &self.edges[*edge_idx];
            self.vertices[edge.hd_vertex].occurs = 0;
            self.vertices[edge.tl_vertex].occurs = 0;
        }

        for edge_idx in face.edges.iter() {
            let edge = &self.edges[*edge_idx];
            self.vertices[edge.hd_vertex].occurs += 1;
            self.vertices[edge.tl_vertex].occurs += 1;
        }

        // Now each occurs value on vertices on this face must be 1 or 2. If
        // it's 1, it's one end of the open polyline
        let mut start = None;
        let mut end = None;
        for edge_idx in face.edges.iter() {
            let edge = &self.edges[*edge_idx];
            if self.vertices[edge.hd_vertex].occurs == 1 {
                if start == None {
                    start = Some(edge.hd_vertex);
                } else if end == None {
                    end = Some(edge.hd_vertex);
                }
            }
            if self.vertices[edge.tl_vertex].occurs == 1 {
                if start == None {
                    start = Some(edge.tl_vertex);
                } else if end == None {
                    end = Some(edge.tl_vertex);
                }
            }
        }

        match (start, end) {
            (Some(start), Some(end)) => Some((start, end)),
            _ => None,
        }
    }
}

struct ClipMeshBuilder<'a> {
    mesh: &'a Mesh,
    cell: &'a Cell,

    vertices: Vec<ClipVertex>,
    edges: Vec<ClipEdge>,
    faces: Vec<ClipFace>,

    vertex_map: HashMap<usize, usize>,
    edge_map: HashMap<usize, usize>,
    face_map: HashMap<usize, usize>,
}

impl<'a> ClipMeshBuilder<'a> {
    fn new(mesh: &'a Mesh, cell_idx: usize) -> Self {
        Self {
            mesh,
            cell: &mesh.cells[cell_idx],

            vertices: Vec::new(),
            edges: Vec::new(),
            faces: Vec::new(),

            vertex_map: HashMap::new(),
            edge_map: HashMap::new(),
            face_map: HashMap::new(),
        }
    }

    fn build(mut self) -> ClipMesh {
        for face_idx in self.cell.faces.iter() {
            self.push_face(*face_idx);
        }

        ClipMesh {
            vertices: self.vertices,
            edges: self.edges,
            faces: self.faces,
        }
    }

    fn push_vertex(&mut self, vertex_idx: usize) -> usize {
        match self.vertex_map.get(&vertex_idx) {
            Some(idx) => *idx,
            None => {
                let fresh_idx = self.vertices.len();
                self.vertices.push(ClipVertex {
                    point: self.mesh.vertices[vertex_idx],
                    distance: 0.0,
                    occurs: 0,
                    visible: true,
                });
                self.vertex_map.insert(vertex_idx, fresh_idx);
                fresh_idx
            }
        }
    }

    fn push_edge(&mut self, edge_idx: usize) -> usize {
        match self.edge_map.get(&edge_idx) {
            Some(idx) => *idx,
            None => {
                let edge = &self.mesh.edges[edge_idx];
                let fresh_idx = self.edges.len();
                let hd_vertex = self.push_vertex(edge.hd_vertex);
                let tl_vertex = self.push_vertex(edge.tl_vertex);
                self.edges.push(ClipEdge {
                    hd_vertex,
                    tl_vertex,
                    faces: SmallVec::new(),
                    visible: true,
                });
                self.edge_map.insert(edge_idx, fresh_idx);
                fresh_idx
            }
        }
    }

    fn push_face(&mut self, face_idx: usize) -> usize {
        match self.face_map.get(&face_idx) {
            Some(idx) => *idx,
            None => {
                let face = &self.mesh.faces[face_idx];
                let fresh_idx = self.faces.len();
                let mut clip_face = ClipFace {
                    edges: SmallVec::new(),
                    visible: true,
                };

                for edge_idx in face.edges.iter() {
                    let clip_edge_idx = self.push_edge(*edge_idx);
                    self.edges[clip_edge_idx].faces.push(fresh_idx);
                    clip_face.edges.push(clip_edge_idx);
                }

                self.faces.push(clip_face);
                self.face_map.insert(face_idx, fresh_idx);

                fresh_idx
            }
        }
    }
}
