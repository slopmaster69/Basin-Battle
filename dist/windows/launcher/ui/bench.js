// Frame benchmark, run by opening the menu with #bench. It measures two things on this PC's
// own GPU and screen:
// 1. Pacing: four real flights between Elwynn and Arathi; every gap between frames, and how
//    many frames the display had to wait for (a gap over 1.5 times its own refresh).
// 2. Headroom: what one frame of the map and its pieces costs when nothing waits for the
//    display, measured by drawing six frames per refresh and waiting for the GPU to finish.
//    1000 / that cost is the highest frame rate this PC could hold.
"use strict";
(() => {
  if (!/bench/.test(location.hash)) return;
  const out = document.createElement("pre");
  Object.assign(out.style, { position: "fixed", left: "50%", top: "56px", transform: "translateX(-50%)", zIndex: 60, margin: 0, padding: "12px 16px", maxWidth: "92vw",
    font: "12.5px/1.45 ui-monospace, Consolas, monospace", color: "#ede6d4", background: "rgba(10,14,16,.9)", border: "1px solid #6b5530", whiteSpace: "pre-wrap" });
  out.id = "bench-out"; out.textContent = "Benchmark: waiting for the map to load…"; document.body.appendChild(out);
  const raf = () => new Promise(r => requestAnimationFrame(r));
  const q = (a, k) => { const s = a.slice().sort((x, y) => x - y); return s[Math.min(s.length - 1, Math.floor(s.length * k))]; };
  async function main() {
    while (!(window.demo && demo.ready())) await new Promise(r => setTimeout(r, 100));
    await new Promise(r => setTimeout(r, 1200));
    const gl = demo.map.gl, dbg = gl && gl.getExtension("WEBGL_debug_renderer_info");
    const gpu = !gl ? "canvas 2d" : dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER);
    // Refresh rate, from 120 idle frames.
    const idle = []; let t = await raf(); for (let i = 0; i < 120; i++) { const n = await raf(); idle.push(n - t); t = n; }
    const refresh = q(idle, .5);
    out.textContent = `Benchmark: flying (display ${(1000 / refresh).toFixed(0)} Hz)…`;
    // 1. Pacing during real flights.
    const gaps = []; let on = true, last = 0; const rec = n => { if (last) gaps.push(n - last); last = n; if (on) requestAnimationFrame(rec); }; requestAnimationFrame(rec);
    for (const g of ["basin", "elwynn", "basin", "elwynn"]) { await demo.pick(g); await new Promise(r => setTimeout(r, 250)); }
    on = false;
    const late = gaps.filter(x => x > refresh * 1.5).length;
    // 2. Headroom: six frames per refresh along a flight's path, then wait for the GPU.
    out.textContent = "Benchmark: measuring headroom…";
    const px = new Uint8Array(4), cost = [], K = 6, a = demo.camNow(), b = { x: 3206, y: 1096, z: .85 };
    for (let f = 0; f < 90; f++) {
      await raf(); const s = performance.now();
      for (let k = 0; k < K; k++) {
        const u = ((f * K + k) % 180) / 180, e = u < .5 ? 2 * u : 2 - 2 * u;
        demo.setCam({ x: a.x + (b.x - a.x) * e, y: a.y + (b.y - a.y) * e, z: Math.exp(Math.log(a.z) + (Math.log(b.z) - Math.log(a.z)) * e - 1.1 * Math.sin(Math.PI * e)) });
        demo.map.state.dirty = true; demo.map.render(performance.now()); drawPieces(performance.now());
      }
      if (gl) gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px); else demo.map.probe();   // waits for the GPU to finish
      cost.push((performance.now() - s) / K);
    }
    await demo.pick("elwynn");
    const c50 = q(cost, .5), c95 = q(cost, .95);
    const res = {
      gpu, display_hz: +(1000 / refresh).toFixed(1), window: `${innerWidth}x${innerHeight} @${devicePixelRatio}x`,
      flight_frames: gaps.length, flight_gap_ms_p50: +q(gaps, .5).toFixed(2), flight_gap_ms_p99: +q(gaps, .99).toFixed(2), flight_gap_ms_max: +q(gaps, 1).toFixed(2), frames_late: late,
      frame_cost_ms_p50: +c50.toFixed(2), frame_cost_ms_p95: +c95.toFixed(2), max_fps_p95: Math.round(1000 / c95),
    };
    window.__bench = res;
    out.textContent = [
      `GPU: ${res.gpu}`, `Display: ${res.display_hz} Hz, window ${res.window}`,
      `Flights: ${res.flight_frames} frames; gap p50 ${res.flight_gap_ms_p50} ms, p99 ${res.flight_gap_ms_p99} ms, max ${res.flight_gap_ms_max} ms; ${late} frames late`,
      `One frame of map + pieces: p50 ${res.frame_cost_ms_p50} ms, p95 ${res.frame_cost_ms_p95} ms (144 Hz allows 6.94 ms)`,
      `Highest steady rate on this PC: ${res.max_fps_p95} fps`,
    ].join("\n");
  }
  main().catch(e => { out.textContent = "Benchmark failed: " + e.message; });
})();
