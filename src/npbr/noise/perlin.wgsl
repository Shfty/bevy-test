#define_import_path npbr::noise::perlin

#import npbr::noise::common

fn gln_perlin(P: vec2<f32>) -> f32 {
  var Pi: vec4<f32> = floor(P.xyxy) + vec4(0.0, 0.0, 1.0, 1.0);
  let  Pf: vec4<f32> = fract(P.xyxy) - vec4(0.0, 0.0, 1.0, 1.0);
  Pi = Pi % 289.0; // To avoid truncation effects in permutation
  var ix: vec4<f32> = Pi.xzxz;
  var iy: vec4<f32> = Pi.yyww;
  var fx: vec4<f32> = Pf.xzxz;
  var fy: vec4<f32> = Pf.yyww;
  var i: vec4<f32> = gln_rand4(gln_rand4(ix) + iy);
  var gx: vec4<f32> = 2.0 * fract(i * 0.0243902439) - 1.0; // 1/41 = 0.024...
  var gy: vec4<f32> = abs(gx) - 0.5;
  var tx: vec4<f32> = floor(gx + 0.5);
  gx = gx - tx;
  var g00: vec2<f32> = vec2(gx.x, gy.x);
  var g10: vec2<f32> = vec2(gx.y, gy.y);
  var g01: vec2<f32> = vec2(gx.z, gy.z);
  var g11: vec2<f32> = vec2(gx.w, gy.w);
  let norm: vec4<f32> =
      1.79284291400159 - 0.85373472095314 * vec4(dot(g00, g00), dot(g01, g01),
                                                 dot(g10, g10), dot(g11, g11));
  g00 *= norm.x;
  g01 *= norm.y;
  g10 *= norm.z;
  g11 *= norm.w;
  let n00 = dot(g00, vec2(fx.x, fy.x));
  let n10 = dot(g10, vec2(fx.y, fy.y));
  let n01 = dot(g01, vec2(fx.z, fy.z));
  let n11 = dot(g11, vec2(fx.w, fy.w));
  let fade_xy = _fade(Pf.xy);
  let n_x = mix(vec2(n00, n01), vec2(n10, n11), fade_xy.x);
  let n_xy = mix(n_x.x, n_x.y, fade_xy.y);
  return 2.3 * n_xy;
}

