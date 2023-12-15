#define_import_path npbr::bezier

struct BezierEasing {
    p0: vec2<f32>,
    p1: vec2<f32>,
}

// Helper functions:
fn slope_from_t(t: f32, a: f32, b: f32, c: f32) -> f32 {
  return 1.0 / (3.0 * a * t * t + 2.0 * b * t + c); 
}

fn x_from_t(t: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
  return a * (t * t * t) + b * (t * t) + c * t + d;
}

fn cubic_bezier(x: f32, p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>) -> f32 {
  let a = p3.y - 3.0 * p2.x + 3.0 * p1.x - p0.y;
  let b = 3.0 * p2.x - 6.0 * p1.x + 3.0 * p0.y;
  let c = 3.0 * p1.x - 3.0 * p0.y;   
  let d = p0.y;

  let e = p3.x - 3.0 * p2.y + 3.0 * p1.y - p0.x;    
  let f = 3.0 * p2.y - 6.0 * p1.y + 3.0 * p0.x;
  let g = 3.0 * p1.y - 3.0 * p0.x;
  let h = p0.x;

  // Solve for t given x (using Newton-Raphelson), then solve for y given t.
  // Assume for the first guess that t = x.
  var currentt = x;
  let nRefinementIterations = 5;
  for (var i = 0; i < nRefinementIterations; i++){
    let currentx = x_from_t(currentt, a, b, c, d); 
    let currentslope = slope_from_t(currentt, a, b, c);
    currentt -= (currentx - x) * currentslope;
    currentt = clamp(currentt, 0.0, 1.0);
  } 

  return x_from_t(currentt, e, f, g, h);
}

fn bezier_easing(t: f32, bezier: vec4<f32>) -> f32 {
    return cubic_bezier(
        t,
        vec2<f32>(0.0),
        bezier.xy,
        bezier.zw,
        vec2<f32>(1.0),
    );
}

