use std::{
    fs::File,
    io::{self, BufReader, Cursor, Read, Seek},
    path::Path,
};

use glam::{Mat4, Vec3};
use stl_io::{read_stl, Normal, Triangle, Vector};

use crate::error::MeshError;

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    fn new(vert: &stl_io::Vertex) -> Self {
        let v = Vec3::new(vert[0], vert[1], vert[2]);
        Self { min: v, max: v }
    }

    fn expand(&mut self, vert: &stl_io::Vertex) {
        let v = Vec3::new(vert[0], vert[1], vert[2]);
        self.min = self.min.min(v);
        self.max = self.max.max(v);
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    fn length(&self) -> f32 {
        self.max.x - self.min.x
    }

    fn width(&self) -> f32 {
        self.max.y - self.min.y
    }

    fn height(&self) -> f32 {
        self.max.z - self.min.z
    }
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub bounds: BoundingBox,
}

impl Mesh {
    /// Load mesh data from file (if provided) or stdin
    pub fn load(model_filename: &str, recalc_normals: bool) -> Result<Self, MeshError> {
        // TODO: Add support for URIs instead of plain file names
        // https://developer.gnome.org/integration-guide/stable/thumbnailer.html.en

        if model_filename == "-" {
            // create_stl_reader requires Seek, so we must read the entire stream into memory before proceeding.
            // So I guess this can just consume all RAM if it gets bad input. Hmmm....
            let mut input_buffer = Vec::new();
            io::stdin().read_to_end(&mut input_buffer)?;
            return Self::from_stl(Cursor::new(input_buffer), recalc_normals);
        }

        let model_filename = Path::new(model_filename);
        let model_file = File::open(model_filename)?;
        match model_filename
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str()
        {
            "obj" => Self::from_obj(model_file, recalc_normals),
            "stl" => Self::from_stl(model_file, recalc_normals),
            "3mf" => Self::from_3mf(model_file, recalc_normals),
            _ => Err(MeshError::UnsupportedFormat),
        }
    }

    pub fn from_3mf<R>(model_file: R, _recalc_normals: bool) -> Result<Self, MeshError>
    where
        R: Read + Seek,
    {
        let models = threemf::read(model_file)?;
        let mut result = None;
        let vertex_translator = |vertex: &threemf::model::Vertex| {
            #[allow(clippy::cast_possible_truncation)]
            stl_io::Vertex::new([vertex.x as f32, vertex.y as f32, vertex.z as f32])
        };

        let mut offset = 0;

        // Combine all the models into a single mesh.
        for model in models {
            for object in model.resources.object {
                let Some(mesh) = &object.mesh else { continue };
                for (i, triangle) in mesh.triangles.triangle.iter().enumerate() {
                    // Re-use `Mesh::process_tri`, which creates new vertices for every
                    // triangle.
                    // Possible optimization: re-use triangles instead.
                    let triangle = Triangle {
                        normal: Normal::new([1.0, 0.0, 0.0]),
                        vertices: [
                            vertex_translator(&mesh.vertices.vertex[triangle.v1]),
                            vertex_translator(&mesh.vertices.vertex[triangle.v2]),
                            vertex_translator(&mesh.vertices.vertex[triangle.v3]),
                        ],
                    };

                    let f_mesh = result.get_or_insert_with(|| Self {
                        vertices: Vec::new(),
                        normals: Vec::new(),
                        indices: Vec::new(),
                        bounds: BoundingBox::new(&triangle.vertices[0]),
                    });

                    f_mesh.process_tri(&triangle, true);
                    f_mesh.indices.extend(
                        (0..3)
                            .map(|j| u32::try_from((offset + i) * 3 + j))
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| MeshError::InvalidThreemf(e.to_string()))?,
                    );
                }
                if let Some(ref mut f_mesh) = result {
                    offset += mesh.triangles.triangle.len();
                    // 3MF files don't have normals, so we need to calculate them.
                    f_mesh.compute_smooth_normals();
                }
            }
        }

        result.ok_or(MeshError::NoMeshData)
    }

    pub fn from_stl<R>(mut model_file: R, recalc_normals: bool) -> Result<Self, MeshError>
    where
        R: Read + Seek,
    {
        let stl = read_stl(&mut model_file)?;
        let mut mesh: Option<Self> = None;
        for (i, face) in stl.faces.iter().enumerate() {
            let triangle = Triangle {
                normal: face.normal,
                vertices: [
                    stl.vertices[face.vertices[0]],
                    stl.vertices[face.vertices[1]],
                    stl.vertices[face.vertices[2]],
                ],
            };

            let mesh = mesh.get_or_insert_with(|| Self {
                vertices: Vec::new(),
                normals: Vec::new(),
                indices: Vec::new(),
                bounds: BoundingBox::new(&triangle.vertices[0]),
            });

            mesh.process_tri(&triangle, recalc_normals);
            for j in 0..3 {
                mesh.indices.push(
                    u32::try_from(i * 3 + j).map_err(|e| MeshError::InvalidStl(e.to_string()))?,
                );
            }
        }

        let mut mesh = mesh.ok_or(MeshError::EmptyMesh)?;

        if mesh.normals.is_empty() {
            mesh.compute_smooth_normals();
        }

        Ok(mesh)
    }

    pub fn from_obj<R>(obj_file: R, _recalc_normals: bool) -> Result<Self, MeshError>
    where
        R: Read,
    {
        let mut model = BufReader::new(obj_file);
        let (models, _) = tobj::load_obj_buf(&mut model, &tobj::GPU_LOAD_OPTIONS, |_| {
            Err(tobj::LoadError::GenericFailure)
        })?;

        let first_mesh = &models.first().ok_or(MeshError::EmptyMesh)?.mesh;
        let mut first_vertex = first_mesh.positions.iter();
        let mut mesh = Self {
            vertices: Vec::with_capacity(first_mesh.positions.len() / 3),
            normals: Vec::with_capacity(first_mesh.normals.len() / 3),
            indices: Vec::with_capacity(first_mesh.indices.len()),
            bounds: BoundingBox::new(&Vector::new([
                *first_vertex.next().ok_or(MeshError::EmptyMesh)?,
                *first_vertex.next().ok_or(MeshError::EmptyMesh)?,
                *first_vertex.next().ok_or(MeshError::EmptyMesh)?,
            ])),
        };

        let mut offset = 0;
        for model in &models {
            let indices = &model.mesh.indices;
            let normals = &model.mesh.normals;
            let positions = &model.mesh.positions;

            mesh.indices.extend(indices.iter().map(|i| i + offset));
            offset += u32::try_from(positions.len() / 3)
                .map_err(|e| MeshError::InvalidObj(e.to_string()))?;

            for vertices in positions
                .chunks_exact(3)
                .map(|v| Vec3::new(v[0], v[1], v[2]))
            {
                mesh.bounds
                    .expand(&Vector::new([vertices.x, vertices.y, vertices.z]));
                mesh.vertices.push(vertices);
            }

            if normals.is_empty() {
                mesh.compute_smooth_normals();
            } else {
                mesh.normals
                    .extend(normals.chunks_exact(3).map(|n| Vec3::new(n[0], n[1], n[2])));
            }
        }
        Ok(mesh)
    }

    // Move the mesh to be centered at the origin
    // and scaled to fit a 2 x 2 x 2 box. This means that
    // all coordinates will be between -1.0 and 1.0
    pub fn scale_and_center(&self) -> Mat4 {
        // Move center to origin
        let center = self.bounds.center();
        let translation_vector = Vec3::new(-center.x, -center.y, -center.z);
        let translation_matrix = Mat4::from_translation(translation_vector);
        // Scale
        let longest = self
            .bounds
            .length()
            .max(self.bounds.width())
            .max(self.bounds.height());
        let scale = 2.0 / longest;
        let scale_matrix = Mat4::from_scale(Vec3::splat(scale));
        scale_matrix * translation_matrix
    }

    fn process_tri(&mut self, tri: &Triangle, recalc_normals: bool) {
        self.vertices.extend(tri.vertices.iter().map(|v| {
            self.bounds.expand(v);
            Vec3::new(v[0], v[1], v[2])
        }));

        // Use normal from STL file if it is provided
        if !(recalc_normals || (tri.normal == Vector::new([0.0, 0.0, 0.0]))) {
            let n = Vec3::new(tri.normal[0], tri.normal[1], tri.normal[2]);
            // TODO: Figure out how to get away with 1 normal instead of 3
            for _ in 0..3 {
                self.normals.push(n);
            }
        }
    }

    /// Calculates the normal of an indexed mesh, smoothing normals for shared vertices.
    ///
    /// Based on code from Bevy's `compute_normals` function.
    /// <https://github.com/bevyengine/bevy/blob/v0.15.0-rc.1/crates/bevy_mesh/src/mesh.rs#L665-L714>
    fn compute_smooth_normals(&mut self) {
        let positions = &self.vertices;
        let mut normals = vec![Vec3::ZERO; positions.len()];

        self.indices.chunks_exact(3).for_each(|face| {
            let [a, b, c] = [face[0] as usize, face[1] as usize, face[2] as usize];

            // Calculate face normal (triangle's normal)
            let normal = {
                let a = positions[a];
                let b = positions[b];
                let c = positions[c];
                (b - a).cross(c - a).normalize()
            };

            // Accumulate the normal to each vertex of the triangle
            normals[a] += normal;
            normals[b] += normal;
            normals[c] += normal;
        });

        // Normalize the vertex normals
        self.normals = normals
            .iter()
            .map(|normal| normal.normalize_or_zero())
            .collect();
    }
}
