use cgmath::Vector4;
use smallvec::SmallVec;

#[derive(Debug, Clone)]
pub struct Edge {
    hd_vertex: usize,
    tl_vertex: usize,
    faces: SmallVec<[usize; 8]>,
}

#[derive(Debug, Clone)]
pub struct Face {
    hd_cell: usize,
    tl_cell: usize,
    edges: SmallVec<[usize; 8]>,
}

#[derive(Debug, Clone)]
pub struct Cell {
    normal: Vector4<f32>,
    faces: SmallVec<[usize; 8]>,
}

pub struct Mesh {
    vertices: Vec<Vector4<f32>>,
    edges: Vec<Edge>,
    faces: Vec<Face>,
    cells: Vec<Cell>,
}

impl Mesh {}
