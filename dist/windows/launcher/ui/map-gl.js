// The launcher's map, drawn by the GPU.
//
// Why: moving the camera must cost almost nothing, so flights hold the display's own rate
// (144 Hz and up). Every tile lives in GPU memory with its mipmaps before the menu is shown, so
// no tile is fetched, decoded or swapped while the camera moves: a frame is one camera uniform
// and at most a dozen textured quads. The candlelight is in the same shader, so no page layer
// blends over the map.
//
//   const map = MapGL.create(canvas, "map/tiles.json", { onReady, onProgress })
//   map.setCamera({ x, y, z })   map point at the anchor, z = screen px per art px
//   map.project([x, y])          -> [screen x, screen y] in CSS px
//   map.render(now)              draws if anything changed (the app's frame loop calls it)
"use strict";
const MapGL = (() => {
  const VS = `#version 300 es
  in vec2 a_pos;
  uniform vec4 u_rect;     // art-space x, y, w, h of this quad
  uniform vec4 u_uv;       // texture u0, v0, u1, v1
  uniform vec3 u_cam;      // art x, y at the anchor; zoom
  uniform vec2 u_anchor;   // CSS px
  uniform vec2 u_view;     // CSS px
  out vec2 v_uv;
  void main() {
    vec2 art = u_rect.xy + a_pos * u_rect.zw;
    vec2 css = (art - u_cam.xy) * u_cam.z + u_anchor;
    gl_Position = vec4(css / u_view * 2.0 - 1.0, 0.0, 1.0) * vec4(1.0, -1.0, 1.0, 1.0);
    v_uv = mix(u_uv.xy, u_uv.zw, a_pos);
  }`;
  const FS = `#version 300 es
  precision mediump float;
  in vec2 v_uv;
  uniform sampler2D u_tex;
  uniform vec2 u_dev;      // device px
  uniform float u_light;   // candle strength, 0 with ambient effects off
  uniform float u_flicker; // the glow's strength this moment, .9 to 1
  out vec4 o;
  // The same two layers the menu had as page overlays, computed here instead, pixel for pixel:
  // a warm glow laid over the map in soft-light (rgba(255,196,120,.42), an ellipse 55% x 60% at
  // 44% 36%, fading out by 72%), then the table's shadow (rgba(16,23,28), an ellipse 66% x 74%
  // at 50% 42%: clear to 38%, .5 at 74%, .88 at the edge and beyond).
  vec3 softLight(vec3 b, vec3 s) {
    vec3 d = mix(sqrt(b), ((16.0 * b - 12.0) * b + 4.0) * b, step(b, vec3(0.25)));
    return mix(b + (2.0 * s - 1.0) * (d - b), b - (1.0 - 2.0 * s) * b * (1.0 - b), step(s, vec3(0.5)));
  }
  void main() {
    vec3 c = texture(u_tex, v_uv).rgb;
    vec2 p = gl_FragCoord.xy / u_dev; p.y = 1.0 - p.y;
    float dw = length((p - vec2(0.44, 0.36)) / vec2(0.55, 0.60));
    float aw = 0.42 * max(0.0, 1.0 - dw / 0.72) * u_light * u_flicker;
    c = mix(c, softLight(c, vec3(1.0, 196.0 / 255.0, 120.0 / 255.0)), aw);
    float dv = length((p - vec2(0.50, 0.42)) / vec2(0.66, 0.74));
    float av = dv < 0.38 ? 0.0 : dv < 0.74 ? 0.5 * (dv - 0.38) / 0.36 : min(0.88, 0.5 + 0.38 * (dv - 0.74) / 0.26);
    c = mix(c, vec3(16.0, 23.0, 28.0) / 255.0, av);
    o = vec4(c, 1.0);
  }`;

  function create(canvas, manifestUrl, opts = {}) {
    const gl = canvas.getContext("webgl2", { alpha: false, antialias: false, depth: false, stencil: false, powerPreference: "high-performance", preserveDrawingBuffer: false });
    if (!gl) throw new Error("WebGL2 isn't available");
    // A lost context (the GPU reset, or out of memory) is reported, and the map drawn again
    // from scratch when it comes back.
    let lost = false;
    canvas.addEventListener("webglcontextlost", e => { e.preventDefault(); lost = true; opts.onLost && opts.onLost(); });
    canvas.addEventListener("webglcontextrestored", () => { opts.onRestored && opts.onRestored(); });
    const sh = (type, src) => { const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s); if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s)); return s; };
    const prog = gl.createProgram(); gl.attachShader(prog, sh(gl.VERTEX_SHADER, VS)); gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, FS)); gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog));
    gl.useProgram(prog);
    const U = {}; for (const n of ["u_rect", "u_uv", "u_cam", "u_anchor", "u_view", "u_dev", "u_tex", "u_light", "u_flicker"]) U[n] = gl.getUniformLocation(prog, n);
    const vao = gl.createVertexArray(); gl.bindVertexArray(vao);
    const vb = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, vb); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]), gl.STATIC_DRAW);
    const loc = gl.getAttribLocation(prog, "a_pos"); gl.enableVertexAttribArray(loc); gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
    gl.uniform1i(U.u_tex, 0);
    const aniso = gl.getExtension("EXT_texture_filter_anisotropic");

    const state = { cam: { x: 0, y: 0, z: 1 }, anchor: [0, 0], view: [1, 1], dpr: 1, dirty: true, light: 1, flicker: 0, ready: false };
    let manifest = null, base = null, tiles = [];
    const toUpload = [];   // decoded bitmaps waiting for the GPU, one a frame

    function texture(bitmap) {
      const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t);
      gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGB8, gl.RGB, gl.UNSIGNED_BYTE, bitmap);
      gl.generateMipmap(gl.TEXTURE_2D);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      if (aniso) gl.texParameterf(gl.TEXTURE_2D, aniso.TEXTURE_MAX_ANISOTROPY_EXT, Math.min(8, gl.getParameter(aniso.MAX_TEXTURE_MAX_ANISOTROPY_EXT)));
      bitmap.close && bitmap.close();
      return t;
    }
    // Decoding happens off the main thread (createImageBitmap); only the upload is on it.
    const decode = url => fetch(url).then(r => { if (!r.ok) throw new Error(url + " " + r.status); return r.blob(); }).then(b => createImageBitmap(b, { premultiplyAlpha: "none", colorSpaceConversion: "none" }));
    const dir = manifestUrl.replace(/[^/]*$/, "");
    fetch(manifestUrl).then(r => r.json()).then(async m => {
      manifest = m;
      base = { tex: texture(await decode(dir + m.base)) }; state.dirty = true;
      tiles = m.tiles.map(t => ({ ...t, tex: null }));
      // The tiles nearest the camera first; all of them before the menu reports ready.
      const order = tiles.slice().sort((a, b) => dist(a) - dist(b));
      await Promise.all(order.map(t => decode(dir + t.file).then(bm => { toUpload.push([t, bm]); })));
      const wait = () => new Promise(r => { const f = () => { if (toUpload.length === 0 && tiles.every(t => t.tex)) r(); else requestAnimationFrame(f); }; f(); });
      await wait(); state.ready = true;
      state.error = gl.getError();   // GL_NO_ERROR (0) if every upload worked
      opts.onReady && opts.onReady();
      function dist(t) { const c = state.cam; return Math.hypot(t.x + 512 - c.x, t.y + 512 - c.y); }
    }).catch(e => { console.error(e); opts.onError && opts.onError(e); });

    function resize() {
      const dpr = Math.min(devicePixelRatio || 1, 2), w = canvas.clientWidth, h = canvas.clientHeight;
      state.dpr = dpr; state.view = [w, h];
      if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) { canvas.width = Math.round(w * dpr); canvas.height = Math.round(h * dpr); }
      state.dirty = true;
    }
    function quad(tex, x, y, w, h, uv) {
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.uniform4f(U.u_rect, x, y, w, h); gl.uniform4f(U.u_uv, uv[0], uv[1], uv[2], uv[3]);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    }
    function render(now) {
      // One upload a frame while loading, so a flight never waits on a big copy.
      if (toUpload.length) { const [t, bm] = toUpload.shift(); t.tex = texture(bm); state.dirty = true; }
      if (!state.dirty || !manifest || !base) return false;
      state.dirty = false;
      const [w, h] = state.view, c = state.cam;
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.uniform3f(U.u_cam, c.x, c.y, c.z); gl.uniform2f(U.u_anchor, state.anchor[0], state.anchor[1]);
      gl.uniform1f(U.u_light, state.light); gl.uniform1f(U.u_flicker, state.flicker);
      gl.uniform2f(U.u_view, w, h); gl.uniform2f(U.u_dev, canvas.width, canvas.height);   // CSS px to place, device px to light
      gl.clearColor(.09, .075, .06, 1); gl.clear(gl.COLOR_BUFFER_BIT);
      const [W, H] = manifest.art, T = manifest.tile, G = manifest.gutter, S = T + 2 * G;
      // Base first (always complete), then the full-detail tiles over it.
      quad(base.tex, 0, 0, W, H, [0, 0, 1, 1]);
      const x0 = c.x - state.anchor[0] / c.z, y0 = c.y - state.anchor[1] / c.z, x1 = x0 + w / c.z, y1 = y0 + h / c.z;
      const inset = [G / S, G / S, (G + T) / S, (G + T) / S];
      for (const t of tiles) {
        if (!t.tex || t.x > x1 || t.x + T < x0 || t.y > y1 || t.y + T < y0) continue;   // off screen
        quad(t.tex, t.x, t.y, T, T, inset);
      }
      return true;
    }
    // The colour drawn in the middle of the screen this moment, read straight back from the GPU.
    function probe() {
      state.dirty = true; render(performance.now());
      const px = new Uint8Array(4); gl.readPixels(canvas.width >> 1, canvas.height >> 1, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
      return Array.from(px).join(",");
    }
    return {
      gl, state, resize, render, probe,
      setCamera(cam, anchor) { state.cam = cam; state.anchor = anchor; state.dirty = true; },
      setLight(light, flicker) { if (light !== state.light || Math.abs(flicker - state.flicker) > .002) { state.light = light; state.flicker = flicker; state.dirty = true; } },
      project([x, y]) { const c = state.cam; return [(x - c.x) * c.z + state.anchor[0], (y - c.y) * c.z + state.anchor[1]]; },
      unproject([sx, sy]) { const c = state.cam; return [(sx - state.anchor[0]) / c.z + c.x, (sy - state.anchor[1]) / c.z + c.y]; },
      get ready() { return state.ready; },
      tilesLoaded() { return tiles.filter(t => t.tex).length + "/" + tiles.length; },
      // What this PC draws with, for the launcher's log.
      about() {
        const d = gl.getExtension("WEBGL_debug_renderer_info");
        return { renderer: d ? gl.getParameter(d.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER), maxTexture: gl.getParameter(gl.MAX_TEXTURE_SIZE),
          error: state.error, lost: lost || gl.isContextLost(), canvas: canvas.width + "x" + canvas.height, tiles: tiles.filter(t => t.tex).length + "/" + tiles.length,
          centre: probe(), style: getComputedStyle(canvas).display + "/" + getComputedStyle(canvas).visibility + "/" + canvas.clientWidth + "x" + canvas.clientHeight };
      },
    };
  }
  return { create };
})();
