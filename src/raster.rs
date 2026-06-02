pub fn edge(a:[f32;3] ,b:[f32;3], p:[f32;3]) -> f32{

    ((p[0] - a[0]) * (b[1] - a[1])) - ((p[1] - a[1]) * (b[0] - a[0]))

}

pub fn draw_triangle(depth_buf: &mut [u32],
      width: usize,
      height: usize,
      v0: [f32; 3],  
      v1: [f32; 3],
      v2: [f32; 3]
    ){

    let min_x = v0[0].min(v1[0]).min(v2[0]).floor().max(0.0) as i32;
    let min_y = v0[1].min(v1[1]).min(v2[1]).floor().max(0.0) as i32;
    let max_x = v0[0].max(v1[0]).max(v2[0]).ceil().min(width  as f32 - 1.0) as i32;
    let max_y = v0[1].max(v1[1]).max(v2[1]).ceil().min(height as f32 - 1.0) as i32;

    let area_into_two = edge(v0, v1, v2);
    if area_into_two == 0.0 {return;}

    for y in min_y..max_y {
        for x in min_x..max_x{
            let p = [x as f32 + 0.5, y as f32 + 0.5, 0.0];

            let e0 = edge(v0, v1, p);
            let e1 = edge(v0, v2, p);
            let e2 = edge(v1, v2, p);

            // inside if all three edges share area2's sign
            let inside = if area_into_two > 0.0 {
                e0 >= 0.0 && e1 >= 0.0 && e2 >= 0.0
            } else {
                e0 <= 0.0 && e1 <= 0.0 && e2 <= 0.0
            };

            if inside {
                let idx = y as usize * width + x as usize;
                depth_buf[idx] = 0xFF_B6_C1;
            }
        }
    }


}