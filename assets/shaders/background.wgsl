#import bevy_pbr::mesh_view_bindings    view, fog, globals
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
const ITS: f32 = 100.0;

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

fn displacement(p: vec3<f32>, k: f32) -> f32 {
    return sin(k*p.x)*sin(k*p.y)*sin(k*p.z);
}

fn scene(p2: vec3<f32>, iTime: f32) -> vec3<f32> {
    let n: f32 = iTime * 1.;
    let txn1: f32 = (sin(n)+PI)/(PI*2.0);

    var p = p2  * (1.0 + sin(n) * 0.01);
    // var p = p2;

    p.x *= 1.0 + sin(n * 1.0 + 0.5) * 0.02;
    // p.y += txnoise(p * 1.0 + sin(n) * 0.5) * 0.5;
    // p.y += txnoise(vec3(n * 1.0, 0.0, 0.0)) * 1.0;
    // p.y += txnoise((p + 1.0) * 10.0) * 0.1;
    // p.x += 0.0;
    // let o1 = 0.1;

    // p.y *= 1.0 + txnoise(vec3(n * 1.0 + p2.x * 5.0, n * 1.0 + p.y, 0.0)) * 1.0;

    
    // p = vec3(p.x, pR(p.yz, (p.x * 4.0 + n * 4.0) * 0.1)); // x axis spiral
    p = vec3(p.x, pR(p.yz, ((p.x + 0.4) * 4.0) * 0.1)); // x axis spiral
    p = vec3(pR(p.xy, (p.z * 2.0 + 1.0) * 0.1), p.z); // spin
    p = vec3(pR(p.xy, (p.z * 2.0 + n * 0.1) * 0.3), p.z); // spin
    
    p = vec3(pR(p.xy, txnoise(p * 2. + n * 0.5) - 0.62), p.z);
    p = vec3(pR(p.xy, txnoise(p * 4. + n * 0.1) - 0.5), p.z);
    p = vec3(p.x, pR(p.yz, n * 0.1).x, pR(p.yz, n * 0.1).y);

    // p -= o1;
    
    let sun_size = 0.9;
    let sphere: f32 = sun_size - length(p) + displacement(p + n * 0.1, 20.0) * 0.05;
    let spherec: vec3<f32> = min(1.0, sphere) * material.color.rgb * 0.1;
    var c: vec3<f32> = spherec;
    c = c * (2500.0 / ITS);
    return c;
}


@fragment
fn fragment(in: MeshVertexOutput) -> @location(0) vec4<f32> {
    //let distance_to_center = distance(in.uv, vec2<f32>(0.5)) * 1.4;
    var uv = in.uv * 2.0 - 1.0;
    //uv.y = -uv.y;
    uv = vec2(uv.y, uv.x);
    let p = vec3<f32>(uv, 0.0) + vec3<f32>(0.1, 0.1, 0.0);
    // uv *= 1.7;
    //uv.x += 0.5;
    // uv.y += 0.4;

    // uv.y += abs(uv.x - 0.0) * 0.2;
    // uv.x += abs(uv.y - 0.0) * 0.2;

    //let speed = 2.0;
    //let t_2 = cos(globals.time * speed);

    // var c: vec3<f32> = vec3<f32>(0.0, 112.0 / 255.0, 1.0) * 0.05;
    var c: vec3<f32> = vec3<f32>(1.0);
    var a: f32 = 0.0;

    let n = globals.time * 2.5;

    // a += txnoise(p * 1000.0 + vec3(0.0, n * 1.0, 0.0));
    // a += step(0.35, txnoise(p * 4000.0 + vec3(0.0, n * 2.0, 0.0))) * 0.2;
    // a += step(0.38, txnoise(p * 2000.0 + vec3(0.0, n * 4.0, 0.0))) * 0.2;
    a += min(1.0, max(0.0, (txnoise(p * 4000.0 + vec3(0.0, n * 2.0, 0.0)) - 0.3) * 4.0));
    a += min(1.0, max(0.0, (txnoise(p * 2000.0 + vec3(0.0, n * 4.0, 0.0)) - 0.4) * 100.0));
    // a += min(1.0, max(0.0, (txnoise(p * 4000.0 + vec3(-n * 2.0, 0.0, 0.0)) - 0.3) * 4.0));
    // a += min(1.0, max(0.0, (txnoise(p * 2000.0 + vec3(-n * 4.0, 0.0, 0.0)) - 0.4) * 100.0));

    // a += min(1.0, max(0.0, (txnoise(p * 100.0 + vec3(0.0, n * 4.0, 0.0)) - 0.2) * 100.0));

    // let ambient = sin(globals.time * 0.4) * 0.5;
    let ambient = sin(p.x);
    a += ambient * 0.1;

    var gp = p;
    let gw = min(1.0, max(0.0, (globals.time * 0.2 - 2.0)));
    // gp.y += sin(gp.y * 15.0) * 0.5;
    var galaxy_gradient = 0.0;
    galaxy_gradient += sin(gp.y * 35.0 + gp.x * 5.0 + 4.0 + globals.time * 0.3 + sin(gp.y * 10.0)) - 0.3;
    galaxy_gradient -= sin(gp.x * 20.0 + gp.y * 100.0) - 1.0;
    // galaxy_gradient *= 1.0 + sin(gp.x * 40.0);
    galaxy_gradient = clamp(galaxy_gradient - 2.0, 0.0, 1.0);

    a += galaxy_gradient * 0.05 * gw;
    c.r -= galaxy_gradient * (1.0 + sin(globals.time * 0.1) * 0.8) * gw;
    c.b -= galaxy_gradient * (0.5 + sin(globals.time * 0.1 + 2.0) * 0.8) * gw;
    

    // c = step(vec3(0.1), c);
    
    // let hr: f32 = 0.05;
    // var z: f32 = -hr;
    // while(z < hr) {
    //     c = c + scene(vec3<f32>(uv, z), globals.time + 10.0);
    //     z = z + 1.0 / ITS;
    // }
    // c = clamp(c, vec3(0.0), vec3(3.0));

    // var alpha = max(0.0, min(1.0, min(1.5, c.r) + min(1.5, c.g) + min(1.5, c.b) - 0.15));
    // let alpha = 1.0;
    

    return vec4<f32>(c, a);
}