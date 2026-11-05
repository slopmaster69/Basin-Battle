# Play's menu: the War Table

What Play shows (`dist/windows/buttons/src/bin/play/`): one page, drawn by WebView2 on Windows
and by the Mac's WebKit in Basin Battle.app (`scripts/mac-launcher.sh`), served from this folder. The release copies it to `launcher\ui` in the Windows download, with the
shield (`dist/windows/art/shield.png`) and `news.json`, the dispatches, written from the
repository's history by `scripts/launcher-news.py`.

- `index.html`: the menu. It asks Play.exe for everything it shows (`host.send`) and is told
  what happened (`launcherRecv`); opened in a browser instead, a stand-in answers (the end of
  the file), so the page can be worked on with `python3 -m http.server` in this folder.
- `map-gl.js`: the map, drawn by the GPU. `map/` is the painted atlas in 24 tiles
  (4096 x 6144 art pixels, north up) and a small whole-map base.
- `bench.js`: `Play.exe -Bench` (or `index.html#bench`) measures how smoothly this PC draws the
  map's flights.
- `ui/`: the campaign art (banners, the brass Play plate, leather, paper, tokens, the Settings
  shell). `fonts/`: Cormorant Garamond and Barlow Semi Condensed (`fonts/OFL.txt`).

The art is made for this project. None of it comes from World of Warcraft's files.
