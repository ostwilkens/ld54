#import bevy_sprite::mesh2d_view_bindings globals
#import bevy_pbr::mesh_vertex_output MeshVertexOutput


struct CustomMaterial {
    color: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> material: CustomMaterial;
@group(1) @binding(1)
var base_color_texture: texture_2d<f32>;
@group(1) @binding(2)
var base_color_sampler: sampler;



const PI: f32 = 3.14159265358979323846;
const purple: vec3<f32> = vec3<f32>(0.298, 0.176, 0.459);
const yellow: vec3<f32> = vec3<f32>(0.675, 0.635, 0.22);
const ITS: f32 = 20.0;

fn txnoise(x: vec3<f32>) -> f32 {
    let p: vec3<f32> = floor(x);
    var f: vec3<f32> = fract(x);
    f = f*f*(3.0 - 2.0*f);
    let uv: vec2<f32> = (p.xy+vec2<f32>(37.0,17.0)*p.z) + f.xy;
    let rg: vec2<f32> = textureSample(base_color_texture, base_color_sampler, (uv+0.5)/256.0, vec2<i32>(0, 0)).yx;
    return mix( rg.x, rg.y, f.z );
}

fn pR(p: vec2<f32>, a: f32) -> vec2<f32> {
    return cos(a)*p + sin(a)*vec2<f32>(p.y, -p.x);
}

fn scene(p2: vec3<f32>, iTime: f32) -> vec3<f32> {
    let n: f32 = iTime * 2.;
    

    var p = p2  * (1.0 + sin(n) * 0.08);
    p.x += 0.0;
    
    p = vec3(p.x, pR(p.yz, (p.x * 4.0 + n * 4.0) * 0.1)); // x axis spiral
    p = vec3(p.x, pR(p.yz, ((p.x + 0.4) * 4.0) * 0.1)); // x axis spiral
    //p = vec3(pR(p.xy, (p.z * 2.0 + 1.0) * 0.1), p.z); // spin
    //p = vec3(pR(p.xy, (p.z * 2.0 + n) * 0.3), p.z); // spin
    
    p = vec3(pR(p.xy, txnoise(p * 2. + n) - 0.62), p.z);
    p = vec3(pR(p.xy, txnoise(p * 4. + n) - 0.5), p.z);
    p = vec3(p.x, pR(p.yz, n).x, pR(p.yz, n).y);
    //p.x = 0.0;
    // p.x = 0.0;
    //p.x -= 0.7;
    p.x *= 0.3;
    // p.x += 0.5;
    
    let thicc = vec3(0.2, 0.2 - p.x, 0.2 - p.x);// + p.x;

    let field: f32 = length(p - clamp(p, -thicc, thicc));
    let cube: f32 = 0.1 / field * 0.05;
    let cubec: vec3<f32> = min(1.0, cube) * material.color.rgb * 0.1;
    var c: vec3<f32> = cubec;
    c = c * (300.0 / ITS);
    return c;
}



@fragment
fn fragment(in: MeshVertexOutput) -> @location(0) vec4<f32> {
    //let distance_to_center = distance(in.uv, vec2<f32>(0.5)) * 1.4;
    var uv = in.uv * 2.0 - 1.0;
    //uv.y = -uv.y;
    uv = vec2(uv.y, uv.x);
    uv.x += 0.1;
    uv *= 1.7;
    //uv.x += 0.5;
    // uv.y += 0.4;

    // uv.y += abs(uv.x - 0.0) * 0.2;
    // uv.x += abs(uv.y - 0.0) * 0.2;

    //let speed = 2.0;
    //let t_2 = cos(globals.time * speed);

    var c: vec3<f32> = vec3<f32>(0.0);
    
    let hr: f32 = 0.05;
    var z: f32 = -hr;
    while(z < hr) {
        c = c + scene(vec3<f32>(uv, z), globals.time + 10.0);
        z = z + 1.0 / ITS;
    }
    c = clamp(c, vec3(0.0), vec3(3.0));


    return vec4<f32>(c, max(0.0, min(1.0, min(1.5, c.r) + min(1.5, c.g) + min(1.5, c.b) - 0.15)));
}