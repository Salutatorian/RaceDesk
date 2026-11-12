# RaceDesk identity assets

The mint checkered flag is the original RaceDesk tray mark, preserved across exports.

- `racedesk-logo.svg` / `.png`: primary mark on the dark app tile.
- `racedesk-symbol.svg`: transparent symbol master.
- `racedesk.ico`: Windows icon containing 16, 24, 32, 48, 64, 128 and 256 px sizes.
- `racedesk-64.rgba`: embedded window icon pixels; no runtime image library is required.
- `variants/`: black, white and mint one-colour SVGs.

Mint: `#6EE7B7`. Dark tile: `#151D28`. Keep the mark square and preserve the checker pattern. For larger presentations use SVG; for Windows use the multi-size ICO.

`generate_branding.py` reproduces the primary assets from the original tray geometry. It requires Pillow for development only; end users do not need it. Vector paths were simplified and audited using the logo-design asset workflow. The mark is intentionally positioned slightly above the tile centre, matching the original tray icon.
