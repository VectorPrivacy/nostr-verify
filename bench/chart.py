"""Draws bench/<platform>-light.svg and -dark.svg from bench/results.json.

    python3 bench/chart.py

One horizontal bar per way of checking a signature: microseconds per signature,
libsecp256k1 one at a time first, then nostr-verify one at a time with a cached
key and in batches of 16 to 1024 signatures by 8 authors. nostr-verify is blue,
libsecp256k1 gray, and every bar is labelled with its name, its value and its
speedup, so color is never the only cue. nostr-parse's palette, which passed
the dataviz validator against GitHub's light and dark pages.
"""

import json
import os
from xml.sax.saxutils import escape

HERE = os.path.dirname(os.path.abspath(__file__))

THEMES = {
    "light": {"ink": "#1f2328", "muted": "#59636e", "rule": "#d1d9e0", "ours": "#2a78d6", "base": "#8e8c85"},
    "dark": {"ink": "#f0f6fc", "muted": "#9198a1", "rule": "#3d444d", "ours": "#3987e5", "base": "#77756f"},
}

LEGEND = [("ours", "nostr-verify"), ("base", "libsecp256k1, one signature at a time")]
FONT = "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif"
WIDTH = 820
LABEL_RIGHT = 262
BAR_LEFT = 274
BAR_MAX = 420
PITCH = 32
BAR = 20
TOP = 104


def bar_path(x, y, w, h, r=4):
    """A bar square at the baseline with a rounded data end."""
    r = min(r, w / 2, h / 2)
    return (
        f"M{x:.1f},{y:.1f} H{x + w - r:.1f} A{r},{r} 0 0 1 {x + w:.1f},{y + r:.1f} "
        f"V{y + h - r:.1f} A{r},{r} 0 0 1 {x + w - r:.1f},{y + h:.1f} H{x:.1f} Z"
    )


def draw(platform, theme):
    t = THEMES[theme]
    base = platform["one_at_a_time"]
    rows = [
        (base, "libsecp256k1 · one at a time", "base"),
        (platform["cached_key"], "one at a time, key cached", "ours"),
    ]
    for n in ("16", "64", "256", "1024"):
        rows.append((platform["batches"][n]["8_keys"], f"batch of {n}", "ours"))
    scale = BAR_MAX / max(us for us, _, _ in rows)
    height = TOP + len(rows) * PITCH + 16

    title = f"Checking Nostr signatures · {platform['label']}"
    subtitle = "Microseconds per BIP-340 signature; batches signed by 8 authors. Lower is better."
    desc = "; ".join(f"{label}: {us:.1f} µs" for us, label, _ in rows)
    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{height}" '
        f'viewBox="0 0 {WIDTH} {height}" role="img" aria-labelledby="t d">',
        f'<title id="t">{escape(title)}</title><desc id="d">{escape(desc)}</desc>',
        f'<g font-family="{FONT}" fill="{t["ink"]}">',
        f'<text x="0" y="22" font-size="18" font-weight="600">{escape(title)}</text>',
        f'<text x="0" y="46" font-size="13" fill="{t["muted"]}">{escape(subtitle)}</text>',
    ]
    x = 0
    for group, text in LEGEND:
        out.append(f'<rect x="{x}" y="64" width="12" height="12" rx="3" fill="{t[group]}"/>')
        out.append(f'<text x="{x + 18}" y="74.5" font-size="12.5" fill="{t["muted"]}">{escape(text)}</text>')
        x += 18 + len(text) * 6.3 + 20
    out.append(
        f'<line x1="{BAR_LEFT - 0.5}" y1="{TOP - 6}" x2="{BAR_LEFT - 0.5}" '
        f'y2="{TOP + len(rows) * PITCH - PITCH + BAR + 6}" stroke="{t["rule"]}" stroke-width="1"/>'
    )
    for i, (us, label, group) in enumerate(rows):
        y = TOP + i * PITCH
        w = max(us * scale, 2)
        weight = ' font-weight="600"' if group == "ours" else ""
        value = f"{us:.1f} µs" if group == "base" else f"{us:.1f} µs · {base / us:.1f}×"
        out.append(
            f'<text x="{LABEL_RIGHT}" y="{y + BAR / 2 + 4.5}" font-size="13.5" '
            f'text-anchor="end"{weight}>{escape(label)}</text>'
        )
        out.append(f'<path d="{bar_path(BAR_LEFT, y, w, BAR)}" fill="{t[group]}"/>')
        out.append(
            f'<text x="{BAR_LEFT + w + 8:.1f}" y="{y + BAR / 2 + 4.5}" font-size="13" '
            f'font-variant-numeric="tabular-nums"{weight}>{escape(value)}</text>'
        )
    out.append("</g></svg>")
    return "\n".join(out) + "\n"


def main():
    with open(os.path.join(HERE, "results.json")) as f:
        results = json.load(f)
    for key, platform in results["platforms"].items():
        for theme in THEMES:
            path = os.path.join(HERE, f"{key}-{theme}.svg")
            with open(path, "w") as f:
                f.write(draw(platform, theme))
            print(path)


if __name__ == "__main__":
    main()
