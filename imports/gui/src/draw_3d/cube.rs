use crate::draw_3d::Vec3;
use crate::gui::Color;
use alloc::vec::Vec;

pub fn draw_cube(
    buffer: &mut [u32],
    buf_w: usize,
    buf_h: usize,
    cam: Vec3,
    fov: f32,
    cube_center: Vec3,
    edge: f32,
    color: Color,
) {
    fn project(
        v: Vec3,
        cam: Vec3,
        fov: f32,
        buf_w: usize,
        buf_h: usize,
    ) -> Option<(i32, i32)> {
        let (dx, dy, dz) = (v.0 - cam.0, v.1 - cam.1, v.2 - cam.2);

        if dz <= 0.0 {
            return None;
        }

        let px = crate::ceil_f32((dx * fov / dz + (buf_w as f32) / 2.0)) as i32;
        let py = crate::ceil_f32((dy * fov / dz + (buf_h as f32) / 2.0)) as i32;

        Some((px, py))
    }

    fn fill_triangle(
        buffer: &mut [u32],
        buf_w: usize,
        buf_h: usize,
        p1: (i32, i32),
        p2: (i32, i32),
        p3: (i32, i32),
        color: Color,
    ) {
        // Simple triangle fill using scanline algorithm
        let mut points = [p1, p2, p3];
        points.sort_by_key(|p| p.1); // Sort by y coordinate

        let (x1, y1) = points[0];
        let (x2, y2) = points[1];
        let (x3, y3) = points[2];

        if y1 == y3 {
            return; // Degenerate triangle
        }

        for y in y1.max(0)..=y3.min(buf_h as i32 - 1) {
            let mut x_left = x1;
            let mut x_right = x1;

            if y1 != y3 {
                // Left edge
                if y <= y2 && y1 != y2 {
                    x_left = x1 + (x2 - x1) * (y - y1) / (y2 - y1);
                } else if y > y2 && y2 != y3 {
                    x_left = x2 + (x3 - x2) * (y - y2) / (y3 - y2);
                }

                // Right edge
                x_right = x1 + (x3 - x1) * (y - y1) / (y3 - y1);
            }

            if x_left > x_right {
                core::mem::swap(&mut x_left, &mut x_right);
            }

            for x in x_left.max(0)..=x_right.min(buf_w as i32 - 1) {
                let idx = (y as usize) * buf_w + (x as usize);
                if idx < buffer.len() {
                    buffer[idx] = color.to_u32();
                }
            }
        }
    }

    let half = edge * 0.5;
    let (cx, cy, cz) = cube_center;
    let verts: [Vec3; 8] = [
        (cx - half, cy - half, cz - half), // 0
        (cx + half, cy - half, cz - half), // 1
        (cx + half, cy + half, cz - half), // 2
        (cx - half, cy + half, cz - half), // 3
        (cx - half, cy - half, cz + half), // 4
        (cx + half, cy - half, cz + half), // 5
        (cx + half, cy + half, cz + half), // 6
        (cx - half, cy + half, cz + half), // 7
    ];

    let projected: Vec<Option<(i32, i32)>> = verts
        .iter()
        .map(|&v| project(v, cam, fov, buf_w, buf_h))
        .collect();

    // Define faces (each face as two triangles)
    let faces = [
        // Front face (z = cz - half)
        ([0, 1, 2], [0, 2, 3]),
        // Back face (z = cz + half)
        ([4, 7, 6], [4, 6, 5]),
        // Left face (x = cx - half)
        ([0, 3, 7], [0, 7, 4]),
        // Right face (x = cx + half)
        ([1, 5, 6], [1, 6, 2]),
        // Bottom face (y = cy - half)
        ([0, 4, 5], [0, 5, 1]),
        // Top face (y = cy + half)
        ([3, 2, 6], [3, 6, 7]),
    ];

    // Draw each face
    for (tri1, tri2) in &faces {
        // Draw first triangle
        if let (Some(p1), Some(p2), Some(p3)) = (projected[tri1[0]], projected[tri1[1]], projected[tri1[2]]) {
            fill_triangle(buffer, buf_w, buf_h, p1, p2, p3, color);
        }

        // Draw second triangle
        if let (Some(p1), Some(p2), Some(p3)) = (projected[tri2[0]], projected[tri2[1]], projected[tri2[2]]) {
            fill_triangle(buffer, buf_w, buf_h, p1, p2, p3, color);
        }
    }
}