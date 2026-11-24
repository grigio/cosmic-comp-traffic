precision highp float;
uniform float alpha;
#if defined(DEBUG_FLAGS)
uniform float tint;
#endif
uniform vec2 size;
varying vec2 v_coords;

uniform vec3 color;
uniform float thickness;
uniform vec4 radius;
uniform float scale;
// How far the element extends past the ring's outer edge. The focus ring is
// padded by FEATHER so the soft falloff is not clipped at the element bounds;
// plain borders (group/pill outlines) pass 0 and behave as before.
uniform float pad;

// Soft-edge width in logical pixels. The stock shader cut the ring off with a
// hard step (only the rounded corners were antialiased); "mac-soft" fades the
// band out over FEATHER px on both sides so the focus ring reads as a glow
// instead of a 1px hard line. Set to 0.0 for the old hard edge.
const float FEATHER = 1.5;

// radius = (top-left, top-right, bottom-right, bottom-left)
float corner_radius(vec2 p, vec4 r) {
    if (p.x < 0.0) {
        return p.y < 0.0 ? r.x : r.w;
    }
    return p.y < 0.0 ? r.y : r.z;
}

// Signed distance to a rounded rectangle centred on the origin (negative
// inside), with per-corner radii.
float rounded_rect_distance(vec2 p, vec2 half_size, vec4 r) {
    float cr = min(corner_radius(p, r), min(half_size.x, half_size.y));
    vec2 q = abs(p) - half_size + cr;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - cr;
}

void main() {
    vec2 location = v_coords * size;
    vec2 half_size = size * 0.5;

    // The ring's own rectangle, i.e. the element shrunk back by `pad`.
    vec2 ring_half_size = max(half_size - vec2(pad), vec2(0.0));
    vec4 ring_radius = max(radius - vec4(pad), vec4(0.0));

    // 0.0 on the ring's outer boundary, negative inside the window.
    float d = rounded_rect_distance(location - half_size, ring_half_size, ring_radius);

    // The ring occupies the `thickness` px just inside the outer boundary.
    // The focus ring (pad > 0) also gets FEATHER of falloff on both edges;
    // plain borders (pad == 0) keep the stock hard edge plus pixel-grid
    // antialiasing, so group/pill outlines are not softened.
    float half_band = max(thickness * 0.5, 0.5 / scale);
    float falloff = pad > 0.0 ? max(FEATHER, 0.5 / scale) : 0.5 / scale;
    float ring_alpha = 1.0 - smoothstep(half_band, half_band + falloff, abs(d + half_band));

    // Pre-multiplied, same as the stock shader: the `color` uniform already
    // has `alpha` folded in by the caller.
    vec4 mix_color = mix(vec4(0.0, 0.0, 0.0, 0.0), vec4(color, alpha), ring_alpha);

    #if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        mix_color = vec4(0.0, 0.3, 0.0, 0.2) + mix_color * 0.8;
    #endif

    gl_FragColor = mix_color;
}
