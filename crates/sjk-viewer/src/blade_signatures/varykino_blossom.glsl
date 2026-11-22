// Varykino Blossom: the spring gardens of Varykino, Naboo's lake retreat, as a blade (Kushina's
// idea, 15/10/2026; the Mythical sibling of the Naboo Blossom). A soft blossom-pink blade, its
// palette locked, with pale petals of light opening and fading along it; lavender and white mist
// curls off it, blossoms drift down round it, and a branch of cherry wood with one flower grows
// from where the blade leaves the hilt. Swinging it sheds real blossoms that flutter to the
// ground behind it (`@petals`, `saber_petals.rs`); a clash bursts a ring of petals.
// Sol's addition (21/10/2026), after the Sakura Droplet of Sekiro: a pearl of nacre hangs at
// its tip, rose and lilac with a rainbow sheen and a white heart (a clash bursts it into the
// petals and it grows back), and the blade itself has the same pearly sheen.
// @part Blossom branch
// @part Lavender mist
// @part Drifting petals
// @part Falling blossoms
// @part Sakura droplet
// @petals {"part":3,"per_unit":0.07,"life":2.6,"size":1.6,"fall":38.0,"drift":14.0}
// @room 9
// @solid 1
// @palette {"corona.rim_cool":[0.72,0.18,0.42],"corona.rim_hot":[1.0,0.55,0.75],"corona.inner":[1.0,0.68,0.84],"core.white":[1.0,0.86,0.93],"core.fringe_cool":[1.0,0.5,0.72],"core.fringe_hot":[1.0,0.7,0.84],"light.color":[1.0,0.55,0.75],"trail":[1.0,0.6,0.8]}
// @extras {"card":"sjk_alpha_014","card_name":"SJK Alpha 014 (Kushina)","finish":"solar"}
// @extra {"id":"blossom_mist","name":"Blossom mist","sheds":{"kind":"blossom_mist","per_unit":0.07,"life":2.6,"size":1.6,"fall":38.0,"drift":14.0}}
// @extra {"id":"ink_stroke","name":"Ink stroke","trail":"ink"}
// @extra {"id":"silk_threads","name":"Silk threads","layer":5}
// @extra {"id":"butterfly","name":"Butterfly","layer":6}
// @extra {"id":"lilac_light","name":"Lilac light","layer":7,"light":[0.75,0.58,1.0],"trail_color":[0.66,0.56,1.0]}

const vec3 VB_PINK = vec3(1.0, 0.52, 0.74);
const vec3 VB_PALE = vec3(1.0, 0.86, 0.92);
const vec3 VB_LILAC = vec3(0.76, 0.64, 0.96);
// the Lilac light extra (`layer` 7, Kushina's, 22/11/2026): the blade's pink light turned to the
// mist's lilac, deeper (not dark), and its pale; little red, so where the blade glows past white
// it stays lavender (Kushina: at 0.72 red its bright parts still turned pink)
const vec3 VB_LILAC_GLOW = vec3(0.58, 0.46, 1.0);
const vec3 VB_LILAC_PALE = vec3(0.8, 0.8, 1.0);
// the Lilac light's solid lilacs (blossoms, bud, petals, the droplet): a soft and a deep one, dark
// enough that a high exposure leaves them lilac, not white (Kushina: the branch's blossoms read
// white and pink)
const vec3 VB_LILAC_SOFT = vec3(0.66, 0.54, 0.96);
const vec3 VB_LILAC_DEEP = vec3(0.46, 0.32, 0.86);

// lilac that stays lilac where the light runs bright (a player's exposure lifts it past white): no
// brighter than VB_LILAC_TOP, and toward that top a little toward lavender, red never past green
// there, so even the igniting tip and its burst at the top stay lilac, not white or pink (Kushina,
// 22/11/2026: at HDR exposure 1.95 the blade, the running light and the burst looked white-pink)
const float VB_LILAC_TOP = 0.8;
vec3 vb_lilac_safe(vec3 c){
  float hi = max(max(c.r, c.g), c.b);
  c *= min(1.0, VB_LILAC_TOP/max(hi, 1e-4));
  hi = min(hi, VB_LILAC_TOP);
  return mix(c, hi*vec3(0.74, 0.74, 1.0), 0.5*smoothstep(0.5*VB_LILAC_TOP, VB_LILAC_TOP, hi));
}
// the droplet's nacre: dusty rose, mauve and cream
const vec3 VB_ROSE = vec3(0.96, 0.6, 0.78);
const vec3 VB_MAUVE = vec3(0.74, 0.48, 0.7);
const vec3 VB_CREAM = vec3(0.96, 0.88, 0.88);
const vec3 VB_WOOD = vec3(0.32, 0.19, 0.12);
const vec3 VB_BARK = vec3(0.58, 0.38, 0.26);

// a five-petal blossom's shape at q (its own plane, radius 1): 0 outside, 1 inside a petal
float vb_flower(vec2 q, float px){
  float a = atan(q.y, q.x);
  float r = length(q);
  // five rounded petals, each with the little notch at its end a cherry blossom has
  float petal = 0.62 + 0.38*abs(cos(2.5*a));
  float notch = 0.12*pow(abs(cos(2.5*a)), 24.0);
  return fillD(r - (petal - notch), px);
}

// the blade: Sol's glow from the file in blossom pink, pale petals of light opening and fading
// along it, a slow breath of white blushing up from the hilt
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float l = lvl(g);
  // the Lilac light extra: lilac where it is pink
  float lil = part(7.0);
  vec3 pink = mix(VB_PINK, VB_LILAC_GLOW, lil), pale = mix(VB_PALE, VB_LILAC_PALE, lil);
  vec3 col = recol(g, pink)*1.15;
  float blush = 0.5 + 0.5*sin(b.y*0.18 - uTime*0.9);
  // (not with the Lilac light extra: its light running up the blade read pink, Kushina)
  col += l*pale*blush*0.35*(1.0 - lil);
  // petals of light in the glow: cells along the blade, each a soft oval that opens and fades
  float cell = floor(b.y*0.45);
  float f = fract(uTime*0.25 + h11(cell*7.1));
  vec2 c = vec2((h11(cell*3.3) - 0.5)*0.9*r, (cell + 0.5)/0.45);
  vec2 q = rot(h11(cell*5.7)*6.2832 + uTime*0.3)*(b - c);
  float pd = length(q/vec2(0.55, 0.3));
  col += l*pale*halo(pd - 0.6, 0.25)*sin(3.1416*f)*0.6;
  // nacre: faint growth lines across the blade, and a soft rainbow sheen at its edges that
  // drifts as a pearl's does when it turns
  float rim = smoothstep(0.25, 0.95, abs(b.x)/max(r, 0.001));
  float lines = 0.5 + 0.5*sin(b.y*1.3 + 4.0*b.x*b.x/max(r*r, 0.001) - uTime*0.4);
  vec3 sheen = hsv(fract(b.y*0.04 + b.x*0.25 + uTime*0.05), 0.45, 1.0);
  // (with the Lilac light extra, a lavender sheen and lilac lines, no pink in them)
  sheen = mix(sheen, hsv(0.62 + 0.1*fract(b.y*0.04 + b.x*0.25 + uTime*0.05), 0.3, 1.0), lil);
  col += l*(sheen*rim*0.3 + mix(VB_CREAM, VB_LILAC_PALE, lil)*lines*0.08);
  // the Lilac light's glow hugs the blade: its light round it (r is the glow's reach, not the
  // blade's) fades out sooner (Kushina: the wide glow made the blade look thick)
  // (not by the hilt, whose round light stays)
  col *= mix(1.0, 1.0 - smoothstep(0.16*r, 0.6*r, abs(b.x)), lil*smoothstep(2.0, 7.0, b.y));
  // and over the core it is a little softer, so the two together stay lilac, not white
  col *= 1.0 - 0.25*lil*(1.0 - smoothstep(0.0, 0.2*r, abs(b.x)));
  // (and a little dimmer, so a high exposure leaves it lilac, not white)
  return mix(col, vb_lilac_safe(col), lil);
}
// the core: blossom pink, or with the Lilac light extra soft lavender, as white as the file's
// core is (its fringe a fuller lavender), red never above green, so where it glows past white it
// turns white, not pink (Kushina, 22/11/2026: red over green turned its bright fringe pink)
vec3 sig_core(vec3 c, vec2 b, float px){
  float hi = max(max(c.r, c.g), c.b);
  float white = min(min(c.r, c.g), c.b)/max(hi, 1e-4);
  // (its heart a light lilac too, not white, and a little dimmer so the lilac shows: Kushina)
  vec3 lilac = hi*0.3*mix(vec3(0.5, 0.44, 1.0), vec3(0.68, 0.64, 1.0), smoothstep(0.4, 0.95, white));
  return mix(c*vec3(1.0, 0.72, 0.86), vb_lilac_safe(lilac), part(7.0));
}

// the file's pink glow round the emitter, lilac with the Lilac light extra
vec3 sig_mouth(vec3 g){ return mix(g, recol(g, VB_LILAC_GLOW), part(7.0)); }

// the light every mythical shares (its awakening surging up the blade, the flare and ring at the
// tip, a clash's ring): lilac with the Lilac light extra, and its slow breath of light rising up
// the blade left out (Kushina, 22/11/2026)
vec3 sig_mythic_light(vec3 l){ return mix(l, vb_lilac_safe(recol(l, VB_LILAC_GLOW)), part(7.0)); }
vec3 sig_mythic_breath(vec3 b){ return b*(1.0 - part(7.0)); }

// blossoms drifting down round the blade: each a five-petal flower facing the camera, turning
// and swaying as it falls, pink or white
vec3 vb_petals(vec3 ro, vec3 rd, float px, float y0, float y1){
  vec3 acc = vec3(0.0);
  if (fxNear(ro, rd, y0 - 4.0, y1) > 4.0) return acc;
  float ta;
  vec3 fu = normalize(cross(rd, abs(rd.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0)));
  vec3 fv = cross(rd, fu);
  for (int i = 0; i < 14; i++){
    float fi = float(i);
    float f = fract(uTime*0.07*(0.7 + 0.6*h11(fi*2.7)) + h11(fi*6.1));
    vec3 p = around(0.8 + 2.0*h11(fi*3.9) + 0.5*sin(uTime*0.8 + fi*2.0), h11(fi*5.3) + 0.04*uTime, mix(y1, y0 - 4.0, f));
    float d = rayPt(ro, rd, p, ta);
    if (d > 1.6 + px) continue;
    vec3 q = ro + rd*ta - p;
    // tumbling: the flower's width shrinks and grows as it turns edge on and back
    float tumble = 0.45 + 0.55*abs(sin(uTime*(0.8 + 0.6*h11(fi*1.9)) + fi));
    vec2 e = rot(uTime*0.9 + fi*1.7)*vec2(dot(q, fu), dot(q, fv));
    float size = 0.55 + 0.2*h11(fi*8.3);
    float shape = vb_flower(e/vec2(size, size*tumble), px/size);
    // (lilac and pale lilac with the Lilac light extra)
    vec3 tint = mix(mix(VB_PINK, VB_PALE, step(0.55, h11(fi*4.4))),
      mix(VB_LILAC_DEEP, VB_LILAC_SOFT, step(0.55, h11(fi*4.4))), part(7.0));
    float life = smoothstep(0.0, 0.08, f)*(1.0 - smoothstep(0.85, 1.0, f));
    tint = mix(tint, vb_lilac_safe(tint), part(7.0));
    acc += (tint*shape*0.9 + tint*halo(d, 0.7)*0.06)*life;
  }
  return acc;
}

// lavender and white mist curling up off the blade, two soft clouds woven together; kept close
// and thin so it does not thicken the blade (Kushina, 20/11/2026: it was 2.8 and 2.0 wide, at
// 1.1 and 0.6)
vec3 vb_mist(vec3 ro, vec3 rd, float y0, float y1){
  return fxSmoke(ro, rd, y0, y1, 1.8, 1.6, 3.0, VB_LILAC)*0.8
    + fxSmoke(ro, rd, y0 + 2.0, y1, 1.3, 2.2, 11.0, mix(vec3(1.0, 0.94, 0.98), VB_LILAC_PALE, part(7.0)))*0.45;
}

// the effects round the blade
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.15, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  fx += vb_mist(ro, rd, y0, y1)*part(1.0);
  fx += vb_petals(ro, rd, px, y0, y1)*part(2.0);
  // the droplet's soft rose light, round it from every side (`rayPt` would hide what is behind
  // the blade's body and cut a hole the blade's shape in it)
  float tp;
  float dp = rayPtRaw(ro, rd, vb_pearl_at(), tp);
  fx += mix(VB_PINK, VB_LILAC_GLOW, part(7.0))*halo(dp, 1.8*max(vb_pearl_size(), 0.2))*0.35
    *part(4.0)*(1.0 - part(6.0));
  // the Butterfly extra's faint light round it
  if (part(6.0) > 0.5){
    float tb;
    float db = rayPtRaw(ro, rd, vb_fly_at(), tb);
    fx += VB_LILAC_PALE*halo(db, 1.6*VB_FLY_R)*0.12;
  }
  // a clash: a ring of petals bursting out across the blade
  if (cl > 0.0){
    float y = L*0.6;
    vec3 ring = vec3(0.0);
    float ta;
    for (int i = 0; i < 10; i++){
      float fi = float(i);
      float R = 0.8 + (1.0 - cl)*4.0;
      vec3 p = around(R, fi/10.0 + 0.05*h11(fi), y + (1.0 - cl)*(h11(fi*2.3) - 0.5)*2.0);
      float d = rayPt(ro, rd, p, ta);
      // (lilac with the Lilac light extra)
      vec3 rc = mix(mix(VB_PINK, VB_PALE, h11(fi*3.1)), mix(VB_LILAC_GLOW, VB_LILAC_SOFT, h11(fi*3.1)), part(7.0));
      ring += rc*(halo(d, 0.28)*0.9 + halo(d, 0.9)*0.15);
    }
    fx += ring*cl*1.3;
  }
  return fx*uFx;
}

// an open blossom at fp, size across, facing the camera, turned by turn: pale at its heart,
// pink at its petals' ends, a gold heart of stamens, over what res holds
vec4 vb_bloom(vec4 res, vec3 ro, vec3 rd, float px, vec3 fp, float size0, float turn){
  float tf;
  float df = rayPt(ro, rd, fp, tf);
  if (df < size0*1.3){
    vec3 q = ro + rd*tf - fp;
    vec3 fu = normalize(cross(rd, abs(rd.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0)));
    vec2 p = rot(turn + 0.15*sin(uTime*0.7))*vec2(dot(q, fu), dot(q, cross(rd, fu)));
    float size = size0*(1.0 + 0.04*sin(uTime*1.3 + turn));
    float shape = vb_flower(p/size, px/size);
    float r = length(p)/size;
    // pale at its heart, pink at its petals' ends, a gold heart of stamens
    // (with the Lilac light extra, pale lilac to lilac)
    vec3 petal = mix(mix(VB_PALE, VB_LILAC_SOFT, part(7.0)), mix(VB_PINK, VB_LILAC_DEEP, part(7.0)),
      smoothstep(0.15, 0.9, r));
    float heart = fillD(r - 0.18, px/size);
    vec3 col = mix(petal, vec3(1.0, 0.8, 0.35), heart);
    res = vec4(mix(res.rgb, col, shape), max(res.a, shape));
  }
  return res;
}

// the droplet's place: a hanging pearl just past the tip; its size, grown back after a clash
// bursts it
vec3 vb_pearl_at(){ return vec3(0.0, uLength + 0.8, 0.0); }
float vb_pearl_size(){ return 0.95*(1.0 - clamp(uClash, 0.0, 1.0))*(1.0 + 0.04*sin(uTime*1.1)); }

// the Sakura Droplet: a teardrop of nacre facing the camera, its point on the blade's tip, rose
// and lilac with growth rings, a rainbow sheen at its rim and a white glow at its heart
vec4 vb_pearl(vec3 ro, vec3 rd, float px){
  float R = vb_pearl_size();
  if (R < 0.05) return vec4(0.0);
  vec3 c = vb_pearl_at();
  float t;
  float d = rayPt(ro, rd, c, t);
  if (d > R*1.8) return vec4(0.0);
  vec3 q = ro + rd*t - c;
  // the blade's direction as seen, so the point stays toward it from every side
  vec3 up = vec3(0.0, 1.0, 0.0) - rd*rd.y;
  up = length(up) > 1e-3 ? normalize(up) : normalize(cross(rd, vec3(1.0, 0.0, 0.0)));
  vec3 side = cross(rd, up);
  // y toward the blade, so its point touches the tip and its round end hangs off it
  vec2 p = vec2(dot(q, side), -dot(q, up))/R;
  // a teardrop: a round base (radius 1) drawn to a soft point 1.2 above its middle, toward the
  // blade, full in the body as the Sakura Droplet is
  float h = clamp(p.y/1.2, 0.0, 1.0);
  float wide = sqrt(max(1.0 - h*h, 0.0))*(1.0 - 0.3*h*h);
  float dist = p.y < 0.0 ? length(p) - 1.0 : (p.y > 1.2 ? 1.0 : abs(p.x) - wide);
  float shape = fillD(dist, px/R);
  if (shape <= 0.0) return vec4(0.0);
  // how near its edge, as a pearl's surface turns away
  float n = clamp(p.y < 0.0 ? length(p) : abs(p.x)/max(wide, 0.05), 0.0, 1.0);
  float edge = pow(n, 3.0);
  // growth rings gathering toward the base, as the droplet's
  float rings = 0.5 + 0.5*sin(length(p - vec2(0.0, -0.9))*14.0 - uTime*0.3);
  rings *= 1.0 - edge;
  // (lilac nacre with the Lilac light extra)
  float lil = part(7.0);
  vec3 col = mix(mix(VB_ROSE, VB_LILAC_SOFT, lil), mix(VB_MAUVE, VB_LILAC_DEEP, lil), edge*0.6)
    + mix(VB_CREAM, VB_LILAC_PALE, lil)*rings*0.12;
  col = mix(col, VB_LILAC, 0.25*smoothstep(0.2, 0.8, p.x + 0.5));
  // the rainbow sheen at its rim, drifting as it turns
  col += hsv(fract(p.x*0.4 + p.y*0.3 + uTime*0.07), 0.7, 1.0)*(edge*0.6 + 0.08);
  // its white heart and a highlight
  col += VB_CREAM*halo(length(p - vec2(0.0, -0.1)), 0.25)*0.45;
  col += vec3(1.0)*halo(length(p - vec2(-0.35, -0.3)), 0.1)*0.7;
  return vec4(min(col, vec3(1.0)), shape*0.92);
}

// the Silk threads extra (`layer` 5, Kushina's, 20/11/2026): a pair of thread-thin silk strands
// wound round the wood of a twig from a to b (w0 to w1 thick), over s0 to s1 of it, turns times
// (a binding), white-lilac with a sheen where they face the eye; how much shows, its colour in col
const vec3 VB_SILK = vec3(0.93, 0.88, 1.0);
const vec3 VB_SILK_SHADE = vec3(0.7, 0.6, 0.86);
float vb_silk(vec3 ro, vec3 rd, float px, vec3 a, vec3 b, float w0, float w1, float s0, float s1,
              float turns, float phase, out vec3 col){
  col = VB_SILK;
  float s;
  float d = raySegRaw(ro, rd, a, b, s);
  float w = mix(w0, w1, s);
  if (d > w || s < s0 || s > s1) return 0.0;
  // where the ray meets the twig's surface, and how far round the twig that is
  vec3 u = normalize(b - a);
  vec3 e1 = normalize(cross(u, abs(u.z) < 0.9 ? vec3(0.0, 0.0, 1.0) : vec3(1.0, 0.0, 0.0)));
  vec3 e2 = cross(u, e1);
  vec3 q = a + (b - a)*s;
  float t = max(dot(q - ro, rd), 0.0);
  vec3 hit = ro + rd*(t - sqrt(max(w*w - d*d, 0.0)));
  vec3 r = hit - q;
  r -= u*dot(r, u);
  float round0 = atan(dot(r, e2), dot(r, e1))/6.2831853;
  // along the binding, the two strands winding round it
  float k = (s - s0)/max(s1 - s0, 1e-4);
  float wound = round0 - k*turns + phase;
  // how far across the strands (not round the twig) the nearest one runs: the winding changes
  // by 1 every girth round and every span/turns along
  float girth = 6.2831853*w;
  float span = length(b - a)*(s1 - s0);
  float across = 1.0/sqrt(1.0/(girth*girth) + turns*turns/max(span*span, 1e-6));
  float d1 = abs(fract(wound + 0.5) - 0.5)*across;
  float d2 = abs(fract(wound) - 0.5)*across;
  float strand = max(fillD(d1 - 0.022, px), fillD(d2 - 0.018, px));
  // tied off at both ends
  strand *= smoothstep(0.0, 0.06, k)*smoothstep(1.0, 0.94, k);
  float facing = clamp(dot(normalize(r), -rd), 0.0, 1.0);
  col = mix(VB_SILK_SHADE, VB_SILK, facing) + vec3(1.0, 0.96, 1.0)*pow(facing, 8.0)*0.35;
  return strand;
}

// the branch: a twig of dark cherry wood growing from where the blade leaves the hilt, curving
// out and up, a smaller twig off it with a bud, and two open blossoms; it circles
// nothing, but stands in true 3D beside the blade
// the branch, its bud and its blossoms
vec4 vb_branch(vec3 ro, vec3 rd, float px){
  if (fxNear(ro, rd, -0.5, 9.0) > 6.5) return vec4(0.0);
  vec3 a = vec3(0.45, 0.3, 0.0);
  vec3 m = vec3(2.1, 3.2, 0.4);
  vec3 e = vec3(3.6, 6.0, 0.25);
  vec3 side = vec3(2.7, 4.3, -1.0);
  float s1, s2, s3;
  float d1 = raySeg(ro, rd, a, m, s1);
  float d2 = raySeg(ro, rd, m, e, s2);
  float d3 = raySeg(ro, rd, m + (e - m)*0.3, side, s3);
  // tapering: thickest at the hilt, thin at the twigs' ends
  float w1 = mix(0.34, 0.24, s1), w2 = mix(0.24, 0.13, s2), w3 = mix(0.15, 0.08, s3);
  float wood = max(max(fillD(d1 - w1, px), fillD(d2 - w2, px)), fillD(d3 - w3, px));
  // bark: lighter along the wood's lit edge, darker rings of grain
  float edge = max(max(d1/w1, d2/w2), 0.0);
  vec3 bark = mix(VB_BARK, VB_WOOD, clamp(edge, 0.0, 1.0))*(0.85 + 0.15*sin((s1 + s2)*14.0));
  vec4 res = vec4(bark, wood);
  // the Silk threads extra: short, dense bindings of silk wound tight here and there on the
  // wood, the bark bare between them (Kushina: dense bindings spread over the twigs, not
  // wound all along); gone with the branch
  if (part(5.0) > 0.5){
    vec3 c, cb;
    float k = vb_silk(ro, rd, px, a, m, 0.34, 0.24, 0.17, 0.29, 8.0, 0.0, c);
    float kb = vb_silk(ro, rd, px, a, m, 0.34, 0.24, 0.62, 0.71, 6.0, 0.4, cb);
    res.rgb = mix(res.rgb, c, k*fillD(d1 - w1, px));
    res.rgb = mix(res.rgb, cb, kb*fillD(d1 - w1, px));
    k = vb_silk(ro, rd, px, m, e, 0.24, 0.13, 0.14, 0.25, 7.0, 0.2, c);
    kb = vb_silk(ro, rd, px, m, e, 0.24, 0.13, 0.52, 0.6, 5.0, 0.7, cb);
    res.rgb = mix(res.rgb, c, k*fillD(d2 - w2, px));
    res.rgb = mix(res.rgb, cb, kb*fillD(d2 - w2, px));
    k = vb_silk(ro, rd, px, m + (e - m)*0.3, side, 0.15, 0.08, 0.38, 0.54, 4.0, 0.5, c);
    res.rgb = mix(res.rgb, c, k*fillD(d3 - w3, px));
  }
  // the bud at the side twig's end
  float tb;
  float db = rayPt(ro, rd, side, tb);
  float bud = fillD(db - 0.32, px);
  vec3 budc = mix(mix(VB_PINK, VB_PALE, 0.3), mix(VB_LILAC_DEEP, VB_LILAC_SOFT, 0.3), part(7.0));
  res = vec4(mix(res.rgb, budc, bud), max(res.a, bud));
  // the open blossoms: one at the branch's end, a smaller one by its fork
  res = vb_bloom(res, ro, rd, px, e + vec3(0.25, 0.4, 0.0), 1.45, 0.3);
  res = vb_bloom(res, ro, rd, px, m + vec3(-0.2, 0.55, 0.5), 0.95, 2.1);
  return res;
}

// the Butterfly extra (`layer` 6, Kushina's, 22/11/2026, after her picture): a white butterfly
// sitting on the tip in the droplet's place, a lilac disc across its wings' roots, its body and
// antennae lilac and the antennae's clubs white; it faces the camera as the droplet does, beats
// its wings slowly now and then, and a clash startles it up off the tip in a little loop, its
// wings beating fast, until it settles back
const vec3 VB_WING = vec3(0.97, 0.96, 1.0);
// its disc, and its body and antennae a little deeper (Kushina: no black on it, all lilac)
const vec3 VB_VIOLET = vec3(0.68, 0.56, 1.0);
const vec3 VB_VIOLET_DEEP = vec3(0.58, 0.44, 0.98);
// half its wingspan, and how long a clash keeps it off the tip (seconds)
const float VB_FLY_R = 1.35;
const float VB_FLIGHT = 1.8;

float vb_smin(float a, float b, float k){
  float h = clamp(0.5 + 0.5*(b - a)/k, 0.0, 1.0);
  return mix(b, a, h) - k*h*(1.0 - h);
}

// how far up in the air it is: 0 on the tip, quickly 1 after a clash, settling back to 0
float vb_air(){
  float u = uClashT/VB_FLIGHT;
  return u >= 1.0 ? 0.0 : smoothstep(0.0, 0.15, u)*(1.0 - smoothstep(0.45, 1.0, u));
}

// where it is: sitting on the tip (its tail at the tip), or in its loop above it
vec3 vb_fly_at(){
  float u = clamp(uClashT/VB_FLIGHT, 0.0, 1.0);
  vec3 loop = vec3(1.2*sin(9.42*u), 2.4, 1.0*sin(6.28*u + 0.8));
  return vec3(0.0, uLength + 0.45*VB_FLY_R, 0.0) + loop*vb_air();
}

// how open its wings are, 1 spread flat: a slow beat now and then on the tip, fast in the air
float vb_open(){
  float ph = fract(uTime/3.4);
  float rest = ph < 0.42 ? 0.5 - 0.5*cos(6.2832*ph/0.42) : 0.0;
  float fast = 0.5 + 0.5*sin(uClashT*6.2832*5.0);
  float a = vb_air();
  return 1.0 - mix(0.6*rest, 0.75*fast, a);
}

// its shape at p (x across, y toward its head, 1 its half wingspan), its wings folded to open of
// their width, seen flat: how much shows, its colour in col
float vb_butterfly_shape(vec2 p, float open, float px, out vec3 col){
  float ax = abs(p.x), wx = ax/open;
  vec2 w2 = vec2(wx, p.y);
  // each side: an upper wing tilted out and up, a rounder lower one, filled in at their root
  float up = sdEll(rot(0.4)*(w2 - vec2(0.6, 0.4)), vec2(0.44, 0.37));
  float lo = sdEll(rot(-0.3)*(w2 - vec2(0.5, -0.38)), vec2(0.4, 0.34));
  float root = sdEll(w2 - vec2(0.24, -0.04), vec2(0.26, 0.15));
  // (distances across the folded wing shrink with it, so its edge stays as sharp)
  float w = vb_smin(vb_smin(up, lo, 0.08), root, 0.08)*open;
  // the V between the upper wings and its head, and the slit between the lower ones and its tail
  float cutUp = (wx - 0.67*p.y - 1.67*pow(max(p.y - 0.4, 0.0), 2.0))*0.83;
  float cutLo = wx - 0.035 - 0.08*max(-0.17 - p.y, 0.0);
  w = -vb_smin(-w, cutUp*open, 0.06);
  w = max(w, -cutLo*open);
  float body = min(length(p - vec2(0.0, 0.165)) - 0.038, sdEll(p - vec2(0.0, 0.02), vec2(0.042, 0.14)));
  // (antennae and tail thick enough to show lilac, not as dark hairlines, at a player's distance)
  float tail = min(sdSeg(p, vec2(0.0, -0.08), vec2(0.0, -0.5)) - 0.036, length(p + vec2(0.0, 0.52)) - 0.046);
  vec2 a2 = vec2(ax, p.y);
  float ant = sdSeg(a2, vec2(0.02, 0.2), vec2(0.145, 0.52)) - 0.02;
  float club = sdEll(rot(-0.35)*(a2 - vec2(0.15, 0.55)), vec2(0.028, 0.046));
  float aw = fillD(w, px), ab = fillD(body, px), at = fillD(tail, px);
  float aa = fillD(ant, px), ac = fillD(club, px);
  // the disc, painted on the wings (folding with them) and filling the slits between them and
  // its body, so nothing dark shows inside it
  float dw = fillD((length(w2) - 0.37)*open, px), db = fillD(length(p) - 0.37, px);
  // a wing folding up turns from the light
  col = mix(VB_WING, VB_VIOLET, dw)*(0.75 + 0.25*open);
  col = mix(col, VB_VIOLET_DEEP, ab);
  col = mix(col, VB_VIOLET_DEEP, at);
  col = mix(col, VB_VIOLET, aa);
  col = mix(col, VB_WING, ac);
  return max(max(max(aw, ab), max(at, aa)), max(ac, dw));
}

vec4 vb_butterfly(vec3 ro, vec3 rd, float px){
  vec3 c = vb_fly_at();
  float t;
  float d = rayPt(ro, rd, c, t);
  if (d > VB_FLY_R*1.25) return vec4(0.0);
  vec3 q = ro + rd*t - c;
  // the blade's direction as seen, its head away from the blade from every side
  vec3 up = vec3(0.0, 1.0, 0.0) - rd*rd.y;
  up = length(up) > 1e-3 ? normalize(up) : normalize(cross(rd, vec3(1.0, 0.0, 0.0)));
  vec3 side = cross(rd, up);
  vec2 p = vec2(dot(q, side), dot(q, up))/VB_FLY_R;
  vec3 col;
  float shape = vb_butterfly_shape(p, vb_open(), px/VB_FLY_R, col);
  return vec4(col, shape*0.97);
}

// what stands round the blade: the branch at the hilt and the droplet at the tip, or the
// Butterfly extra's butterfly in its place
vec4 sig_orbit_solid(vec3 ro, vec3 rd, float px){
  float fly = part(6.0);
  vec4 drop = vb_pearl(ro, rd, px)*part(4.0)*(1.0 - fly);
  vec4 branch = vb_branch(ro, rd, px)*part(0.0);
  vec4 res = vec4(mix(branch.rgb, drop.rgb, drop.a), max(branch.a, drop.a));
  vec4 bf = fly > 0.5 ? vb_butterfly(ro, rd, px) : vec4(0.0);
  return vec4(mix(res.rgb, bf.rgb, bf.a), max(res.a, bf.a));
}
