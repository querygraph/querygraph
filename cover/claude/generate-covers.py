"""Generate the QueryGraph Stack headboard and book cover as SVG.

Scene: aurora borealis made of a querygraph network (nodes, edges, pulses)
flowing over a skyline of skyscrapers and cathedrals, with the Q logo as the
centrepiece. Everything is procedural with a fixed seed so re-renders match.
"""
import math, random, sys, pathlib

CYAN = "#35D9EE"; CYAN2 = "#7FEFFB"; GREEN = "#7CF5C0"; MINT = "#9CF5BF"
VIOLET = "#9B6CFF"; MAGENTA = "#E07CFF"; LILAC = "#B48BFF"; WHITE = "#EFFFFF"
AMBER = "#FFC46B"; AMBER2 = "#FFE2A8"


def f(x):
    return f"{x:.1f}".rstrip("0").rstrip(".")


class Scene:
    def __init__(self, W, H, horizon, seed):
        self.W, self.H, self.hz = W, H, horizon
        self.r = random.Random(seed)
        self.defs, self.body = [], []
        self.gid = 0
        self.landmarks = []   # (x, y) tips to link from the network
        self.net_nodes = []   # network nodes (x, y)

    def uid(self, p):
        self.gid += 1
        return f"{p}{self.gid}"

    # ------------------------------------------------------------ background
    def sky(self):
        W, H, hz = self.W, self.H, self.hz
        self.defs.append(f'''<linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="#03050C"/><stop offset="0.45" stop-color="#070C1E"/>
  <stop offset="0.8" stop-color="#0B1738"/><stop offset="1" stop-color="#123055"/></linearGradient>
<radialGradient id="hglow" cx="0.5" cy="1" r="0.75">
  <stop offset="0" stop-color="#2A7FA8" stop-opacity="0.55"/><stop offset="0.5" stop-color="#1A4A78" stop-opacity="0.2"/>
  <stop offset="1" stop-color="#0B1738" stop-opacity="0"/></radialGradient>''')
        self.body.append(f'<rect width="{W}" height="{hz}" fill="url(#sky)"/>')
        self.body.append(f'<rect x="0" y="{f(hz*0.35)}" width="{W}" height="{f(hz*0.65)}" fill="url(#hglow)"/>')

    def stars(self, n):
        r = self.r
        out = []
        for _ in range(n):
            x, y = r.uniform(0, self.W), r.uniform(0, self.hz * 0.92)
            rad = r.choice([0.7, 0.9, 1.1, 1.4, 1.8, 2.4])
            op = r.uniform(0.25, 0.95)
            col = r.choice([WHITE, WHITE, "#CFEFFF", "#E8DFFF"])
            out.append(f'<circle cx="{f(x)}" cy="{f(y)}" r="{rad}" fill="{col}" opacity="{f(op)}"/>')
            if rad >= 1.8:
                out.append(f'<circle cx="{f(x)}" cy="{f(y)}" r="{rad*4}" fill="{col}" opacity="0.08"/>')
        self.body.append('<g>' + "".join(out) + '</g>')

    # ---------------------------------------------------------------- aurora
    def curtain(self, base, amp, wave, phase, thick, ray_len, hue, top_hue, x0=None, x1=None):
        r = self.r
        W = self.W
        x0 = -60 if x0 is None else x0
        x1 = W + 60 if x1 is None else x1

        def spine(x):
            return (base + amp * math.sin(x / wave + phase)
                    + amp * 0.45 * math.sin(x / (wave * 0.37) + phase * 2.1)
                    + amp * 0.2 * math.sin(x / (wave * 0.13) + phase * 0.7))

        def thickness(x):
            return thick * (0.65 + 0.35 * math.sin(x / (wave * 0.8) + phase + 1.3))

        xs = list(range(int(x0), int(x1) + 1, 12))
        top = [(x, spine(x) - thickness(x)) for x in xs]
        bot = [(x, spine(x) + thickness(x) * 0.55) for x in xs]
        ribbon = "M" + " L".join(f"{f(x)},{f(y)}" for x, y in top) + " L" + " L".join(f"{f(x)},{f(y)}" for x, y in reversed(bot)) + " Z"
        fringe_top = [(x, spine(x) - thickness(x) * 2.4) for x in xs]
        fringe = "M" + " L".join(f"{f(x)},{f(y)}" for x, y in fringe_top) + " L" + " L".join(f"{f(x)},{f(y)}" for x, y in reversed(top)) + " Z"
        core = " ".join(f"{'M' if i == 0 else 'L'}{f(x)},{f(y)}" for i, (x, y) in enumerate([(x, spine(x) + thickness(x) * 0.2) for x in xs]))

        g = []
        g.append(f'<path d="{fringe}" fill="{top_hue}" opacity="0.22" filter="url(#blur60)"/>')
        g.append(f'<path d="{ribbon}" fill="{hue}" opacity="0.42" filter="url(#blur34)"/>')
        g.append(f'<path d="{core}" fill="none" stroke="{GREEN}" stroke-width="{f(thick*0.5)}" opacity="0.5" filter="url(#blur16)"/>')
        g.append(f'<path d="{core}" fill="none" stroke="#D8FFF0" stroke-width="3" opacity="0.5" filter="url(#blur4)"/>')
        # rays: thin vertical streaks falling from the spine
        rays = []
        x = x0
        while x < x1:
            L = ray_len * r.uniform(0.35, 1.0)
            w = r.uniform(1.5, 5.5)
            y = spine(x) - thickness(x) * r.uniform(0.2, 0.9)
            drift = r.uniform(-10, 10)
            col = r.choice([hue, GREEN, CYAN2, top_hue])
            rays.append(f'<line x1="{f(x)}" y1="{f(y)}" x2="{f(x+drift)}" y2="{f(y+L)}" stroke="{col}" stroke-width="{f(w)}" opacity="{f(r.uniform(0.1, 0.38))}"/>')
            x += r.uniform(5, 14)
        mid = self.uid("rm")
        ymin = base - amp - thick * 1.2
        ymax = base + amp + ray_len
        self.defs.append(f'<linearGradient id="{mid}g" gradientUnits="userSpaceOnUse" x1="0" y1="{f(ymin)}" x2="0" y2="{f(ymax)}">'
                         f'<stop offset="0" stop-color="#fff" stop-opacity="0"/><stop offset="0.25" stop-color="#fff" stop-opacity="1"/>'
                         f'<stop offset="0.55" stop-color="#fff" stop-opacity="0.55"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>'
                         f'<mask id="{mid}"><rect x="-100" y="{f(ymin)}" width="{W+200}" height="{f(ymax-ymin)}" fill="url(#{mid}g)"/></mask>')
        g.append(f'<g mask="url(#{mid})" filter="url(#blur3)">{"".join(rays)}</g>')
        self.body.append('<g style="mix-blend-mode:screen">' + "".join(g) + '</g>')

        # network nodes along the spine
        nodes = []
        x = x0 + r.uniform(20, 80)
        while x < x1 - 20:
            nodes.append((x, spine(x) + r.uniform(-thick * 0.7, thick * 0.7)))
            x += r.uniform(95, 175)
        return nodes

    def network(self, layers, extra_links=0.3):
        r = self.r
        edges = []
        for layer in layers:
            for a, b in zip(layer, layer[1:]):
                edges.append((a, b))
        for i in range(len(layers) - 1):
            A, B = layers[i], layers[i + 1]
            for a in A:
                if r.random() < extra_links:
                    b = min(B, key=lambda p: (p[0] - a[0]) ** 2 + (p[1] - a[1]) ** 2)
                    if math.hypot(b[0] - a[0], b[1] - a[1]) < 420:
                        edges.append((a, b))
        self.draw_edges(edges, op=0.55, w=1.6)
        for layer in layers:
            for (x, y) in layer:
                self.node(x, y, r.uniform(3, 5.2))
        self.net_nodes = [p for layer in layers for p in layer]

    def draw_edges(self, edges, op=0.5, w=1.5, pulse_p=0.45):
        r = self.r
        g, pulses = [], []
        for (a, b) in edges:
            g.append(f'<line x1="{f(a[0])}" y1="{f(a[1])}" x2="{f(b[0])}" y2="{f(b[1])}"/>')
            if r.random() < pulse_p:
                t = r.uniform(0.2, 0.8)
                pulses.append(self.pulse_on(a, b, t))
        self.body.append(f'<g stroke="{CYAN2}" stroke-width="{w}" opacity="{op}" stroke-linecap="round">{"".join(g)}</g>')
        self.body.append("".join(pulses))

    def pulse_on(self, a, b, t, size=3.2):
        px, py = a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t
        tx, ty = a[0] + (b[0] - a[0]) * (t - 0.09), a[1] + (b[1] - a[1]) * (t - 0.09)
        return (f'<line x1="{f(tx)}" y1="{f(ty)}" x2="{f(px)}" y2="{f(py)}" stroke="#DDFFFF" stroke-width="{f(size*0.9)}" opacity="0.55" stroke-linecap="round"/>'
                f'<circle cx="{f(px)}" cy="{f(py)}" r="{f(size*2.6)}" fill="{CYAN2}" opacity="0.22"/>'
                f'<circle cx="{f(px)}" cy="{f(py)}" r="{f(size)}" fill="#FFFFFF" opacity="0.95"/>')

    def node(self, x, y, rad, col=WHITE, ring=CYAN):
        self.body.append(f'<circle cx="{f(x)}" cy="{f(y)}" r="{f(rad*3.4)}" fill="{ring}" opacity="0.16"/>'
                         f'<circle cx="{f(x)}" cy="{f(y)}" r="{f(rad*1.9)}" fill="{ring}" opacity="0.35"/>'
                         f'<circle cx="{f(x)}" cy="{f(y)}" r="{f(rad)}" fill="{col}"/>')

    # ------------------------------------------------------------------ logo
    def logo(self, cx, cy, R, glow=True):
        r = self.r
        sw_o, sw_i = R * 0.105, R * 0.085
        Ri = R * 0.65
        g = []
        hid = self.uid("hg")
        ang = math.radians(45)
        hx0, hy0 = cx + math.cos(ang) * R * 0.98, cy + math.sin(ang) * R * 0.98
        hx1, hy1 = cx + math.cos(ang) * R * 1.95, cy + math.sin(ang) * R * 1.95
        self.defs.append(f'<linearGradient id="{hid}" gradientUnits="userSpaceOnUse" x1="{f(hx0)}" y1="{f(hy0)}" x2="{f(hx1)}" y2="{f(hy1)}">'
                         f'<stop offset="0" stop-color="{CYAN}"/><stop offset="0.55" stop-color="#6FC9F7"/><stop offset="1" stop-color="{LILAC}"/></linearGradient>'
                         f'<radialGradient id="{hid}r"><stop offset="0" stop-color="{CYAN}" stop-opacity="0.35"/><stop offset="0.6" stop-color="{CYAN}" stop-opacity="0.08"/><stop offset="1" stop-color="{CYAN}" stop-opacity="0"/></radialGradient>')
        if glow:
            g.append(f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(R*1.75)}" fill="url(#{hid}r)"/>')
            g.append(f'<g filter="url(#blur16)" opacity="0.75">'
                     f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(R)}" fill="none" stroke="{CYAN}" stroke-width="{f(sw_o*1.6)}"/>'
                     f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(Ri)}" fill="none" stroke="{CYAN}" stroke-width="{f(sw_i*1.6)}"/>'
                     f'<line x1="{f(hx0)}" y1="{f(hy0)}" x2="{f(hx1)}" y2="{f(hy1)}" stroke="{LILAC}" stroke-width="{f(sw_o*1.8)}" stroke-linecap="round"/></g>')
        # lens: faint glass tint
        g.append(f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(Ri - sw_i/2)}" fill="#0A1A3A" opacity="0.28"/>')
        # spokes
        for k in range(8):
            a = math.radians(k * 45 - 90)
            g.append(f'<line x1="{f(cx+math.cos(a)*Ri)}" y1="{f(cy+math.sin(a)*Ri)}" x2="{f(cx+math.cos(a)*R)}" y2="{f(cy+math.sin(a)*R)}" stroke="{CYAN2}" stroke-width="{f(R*0.009)}" opacity="0.75"/>')
        # handle
        g.append(f'<line x1="{f(hx0)}" y1="{f(hy0)}" x2="{f(hx1)}" y2="{f(hy1)}" stroke="url(#{hid})" stroke-width="{f(sw_o*1.15)}" stroke-linecap="round"/>')
        g.append(f'<line x1="{f(hx0 + math.cos(ang)*R*0.1)}" y1="{f(hy0 + math.sin(ang)*R*0.1 - sw_o*0.22)}" x2="{f(hx1 - math.cos(ang)*R*0.12)}" y2="{f(hy1 - math.sin(ang)*R*0.12 - sw_o*0.22)}" stroke="#FFFFFF" stroke-width="{f(sw_o*0.14)}" stroke-linecap="round" opacity="0.5"/>')
        g.append(f'<circle cx="{f(hx1)}" cy="{f(hy1)}" r="{f(sw_o*0.26)}" fill="#F2EAFF" opacity="0.9"/>')
        # rings
        g.append(f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(R)}" fill="none" stroke="{CYAN}" stroke-width="{f(sw_o)}"/>')
        g.append(f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(Ri)}" fill="none" stroke="{CYAN}" stroke-width="{f(sw_i)}"/>')
        g.append(f'<circle cx="{f(cx)}" cy="{f(cy)}" r="{f(R + sw_o*0.18)}" fill="none" stroke="#FFFFFF" stroke-width="{f(sw_o*0.1)}" opacity="0.35"/>')
        # travelling pulses on the rings (arcs)
        for (rr, a0, span) in [(R, 200, 26), (R, 335, 18), (Ri, 110, 30), (Ri, 290, 16)]:
            a1 = a0 + span
            p0 = (cx + math.cos(math.radians(a0)) * rr, cy + math.sin(math.radians(a0)) * rr)
            p1 = (cx + math.cos(math.radians(a1)) * rr, cy + math.sin(math.radians(a1)) * rr)
            g.append(f'<path d="M{f(p0[0])},{f(p0[1])} A{f(rr)},{f(rr)} 0 0 1 {f(p1[0])},{f(p1[1])}" fill="none" stroke="#E6FFFF" stroke-width="{f(sw_i*0.45)}" stroke-linecap="round" opacity="0.8" filter="url(#blur3)"/>')
            g.append(f'<circle cx="{f(p1[0])}" cy="{f(p1[1])}" r="{f(sw_i*0.42)}" fill="#FFFFFF"/>')
        # nodes
        for k in range(8):
            a = math.radians(k * 45 - 90)
            ox, oy = cx + math.cos(a) * R, cy + math.sin(a) * R
            ix, iy = cx + math.cos(a) * Ri, cy + math.sin(a) * Ri
            g.append(f'<circle cx="{f(ox)}" cy="{f(oy)}" r="{f(sw_o*0.62)}" fill="#0B2A3C" opacity="0.5"/>'
                     f'<circle cx="{f(ox)}" cy="{f(oy)}" r="{f(sw_o*0.42)}" fill="{WHITE}"/>'
                     f'<circle cx="{f(ix)}" cy="{f(iy)}" r="{f(sw_i*0.62)}" fill="#0B2A3C" opacity="0.5"/>'
                     f'<circle cx="{f(ix)}" cy="{f(iy)}" r="{f(sw_i*0.46)}" fill="{MINT}"/>'
                     f'<circle cx="{f(ix)}" cy="{f(iy)}" r="{f(sw_i*0.2)}" fill="#FFFFFF"/>')
        self.body.append("".join(g))
        self.logo_nodes = [(cx + math.cos(math.radians(k * 45 - 90)) * R, cy + math.sin(math.radians(k * 45 - 90)) * R) for k in range(8)]

    # ------------------------------------------------------------------ city
    def skyscraper(self, x, w, h, scale, lit=True, far=False):
        r = self.r
        hz = self.hz
        g = []
        fill = "#0F1C3C" if far else "#05070F"
        top = hz - h
        blocks = [(x, top, w, h)]
        if r.random() < 0.6 and w > 50 * scale:
            h2 = h * r.uniform(0.12, 0.3)
            w2 = w * r.uniform(0.45, 0.7)
            blocks.append((x + (w - w2) / 2, top - h2, w2, h2))
            top -= h2
            if r.random() < 0.4:
                h3 = h2 * 0.6; w3 = w2 * 0.5
                blocks.append((x + (w - w3) / 2, top - h3, w3, h3)); top -= h3
        for (bx, by, bw, bh) in blocks:
            g.append(f'<rect x="{f(bx)}" y="{f(by)}" width="{f(bw)}" height="{f(bh)}" fill="{fill}"/>')
        tipx = x + w / 2
        tipy = top
        if r.random() < 0.75:
            ah = r.uniform(30, 90) * scale
            g.append(f'<line x1="{f(tipx)}" y1="{f(top)}" x2="{f(tipx)}" y2="{f(top-ah)}" stroke="{"#1A2B55" if far else "#0B1430"}" stroke-width="{f(max(1.5, 3*scale))}"/>')
            tipy = top - ah
            if not far:
                g.append(f'<circle cx="{f(tipx)}" cy="{f(tipy)}" r="{f(3.5*scale)}" fill="#FF7A6B" opacity="0.9"/>'
                         f'<circle cx="{f(tipx)}" cy="{f(tipy)}" r="{f(9*scale)}" fill="#FF7A6B" opacity="0.18"/>')
        if not far:
            # rim light on roof and left edge
            for (bx, by, bw, bh) in blocks:
                g.append(f'<line x1="{f(bx)}" y1="{f(by)}" x2="{f(bx+bw)}" y2="{f(by)}" stroke="{CYAN2}" stroke-width="1.2" opacity="0.5"/>'
                         f'<line x1="{f(bx)}" y1="{f(by)}" x2="{f(bx)}" y2="{f(by+bh)}" stroke="{CYAN}" stroke-width="1" opacity="0.22"/>')
            if lit:
                win = []
                cw, chh = 13 * scale, 17 * scale
                for (bx, by, bw, bh) in blocks:
                    cols = max(1, int((bw - 8 * scale) // cw)); rows = max(1, int((bh - 10 * scale) // chh))
                    ox = bx + (bw - cols * cw) / 2 + cw * 0.25
                    for i in range(cols):
                        for j in range(rows):
                            if r.random() < 0.5:
                                col = r.choice(["#CFE9FF", "#CFE9FF", CYAN2, "#FFF1C9"])
                                win.append(f'<rect x="{f(ox+i*cw)}" y="{f(by+6*scale+j*chh)}" width="{f(cw*0.5)}" height="{f(chh*0.5)}" fill="{col}" opacity="{f(r.uniform(0.25,0.85))}"/>')
                g.append("".join(win))
        return "".join(g), (tipx, tipy)

    def cathedral(self, x, w, scale, far=False):
        r = self.r
        hz = self.hz
        fill = "#0F1C3C" if far else "#05070F"
        tw = w * 0.2
        bh = w * 0.62
        th = w * 1.35
        sh = w * 0.55
        gh = w * 0.28
        g = []
        # body + gable + towers + spires
        g.append(f'<rect x="{f(x+tw*0.9)}" y="{f(hz-bh)}" width="{f(w-tw*1.8)}" height="{f(bh)}" fill="{fill}"/>')
        g.append(f'<polygon points="{f(x+tw*0.9)},{f(hz-bh)} {f(x+w/2)},{f(hz-bh-gh)} {f(x+w-tw*0.9)},{f(hz-bh)}" fill="{fill}"/>')
        tips = []
        for tx in (x, x + w - tw):
            g.append(f'<rect x="{f(tx)}" y="{f(hz-th)}" width="{f(tw)}" height="{f(th)}" fill="{fill}"/>')
            g.append(f'<polygon points="{f(tx-tw*0.08)},{f(hz-th)} {f(tx+tw/2)},{f(hz-th-sh)} {f(tx+tw*1.08)},{f(hz-th)}" fill="{fill}"/>')
            # pinnacles on tower corners
            for px in (tx + tw * 0.08, tx + tw * 0.92):
                g.append(f'<polygon points="{f(px-tw*0.09)},{f(hz-th+2)} {f(px)},{f(hz-th-sh*0.28)} {f(px+tw*0.09)},{f(hz-th+2)}" fill="{fill}"/>')
            # finial + cross
            fx, fy = tx + tw / 2, hz - th - sh
            g.append(f'<line x1="{f(fx)}" y1="{f(fy)}" x2="{f(fx)}" y2="{f(fy-w*0.07)}" stroke="{fill}" stroke-width="{f(2.5*scale)}"/>'
                     f'<line x1="{f(fx-w*0.02)}" y1="{f(fy-w*0.05)}" x2="{f(fx+w*0.02)}" y2="{f(fy-w*0.05)}" stroke="{fill}" stroke-width="{f(2.5*scale)}"/>')
            tips.append((fx, fy - w * 0.07))
        # flying buttresses
        for i in range(3):
            bx = x + tw * 0.9 - w * 0.03
            by = hz - bh * (0.35 + i * 0.2)
            g.append(f'<path d="M{f(bx)},{f(hz)} L{f(bx)},{f(by+bh*0.1)} Q{f(bx+w*0.03)},{f(by-bh*0.05)} {f(bx+w*0.06)},{f(by-bh*0.08)}" fill="none" stroke="{fill}" stroke-width="{f(5*scale)}"/>')
        # central spire (flèche)
        cx = x + w / 2
        g.append(f'<polygon points="{f(cx-w*0.035)},{f(hz-bh-gh*0.6)} {f(cx)},{f(hz-bh-gh-w*0.42)} {f(cx+w*0.035)},{f(hz-bh-gh*0.6)}" fill="{fill}"/>')
        tips.append((cx, hz - bh - gh - w * 0.42))
        if far:
            return "".join(g), tips
        # rim light
        g.append(f'<g stroke="{CYAN2}" stroke-width="1.1" opacity="0.45" fill="none">'
                 f'<polyline points="{f(x+tw*0.9)},{f(hz-bh)} {f(x+w/2)},{f(hz-bh-gh)} {f(x+w-tw*0.9)},{f(hz-bh)}"/>'
                 + "".join(f'<polyline points="{f(tx-tw*0.08)},{f(hz-th)} {f(tx+tw/2)},{f(hz-th-sh)} {f(tx+tw*1.08)},{f(hz-th)}"/>' for tx in (x, x + w - tw))
                 + '</g>')
        # warm glow behind facade
        g.append(f'<ellipse cx="{f(cx)}" cy="{f(hz-bh*0.45)}" rx="{f(w*0.5)}" ry="{f(bh*0.6)}" fill="{AMBER}" opacity="0.1" filter="url(#blur34)"/>')
        # rose window
        rx, ry, rr = cx, hz - bh * 0.72, w * 0.075
        g.append(f'<circle cx="{f(rx)}" cy="{f(ry)}" r="{f(rr*1.9)}" fill="{AMBER}" opacity="0.14"/>'
                 f'<circle cx="{f(rx)}" cy="{f(ry)}" r="{f(rr)}" fill="{AMBER}" opacity="0.9"/>'
                 f'<circle cx="{f(rx)}" cy="{f(ry)}" r="{f(rr*0.55)}" fill="none" stroke="{fill}" stroke-width="{f(1.6*scale)}"/>')
        for k in range(8):
            a = math.radians(k * 45)
            g.append(f'<line x1="{f(rx)}" y1="{f(ry)}" x2="{f(rx+math.cos(a)*rr)}" y2="{f(ry+math.sin(a)*rr)}" stroke="{fill}" stroke-width="{f(1.8*scale)}"/>')
        # pointed-arch windows: nave row + tower lancets
        def arch(ax, ay, aw, ah, col=AMBER, op=0.85):
            return (f'<path d="M{f(ax)},{f(ay+ah)} L{f(ax)},{f(ay+aw*0.5)} Q{f(ax)},{f(ay)} {f(ax+aw/2)},{f(ay)} Q{f(ax+aw)},{f(ay)} {f(ax+aw)},{f(ay+aw*0.5)} L{f(ax+aw)},{f(ay+ah)} Z" fill="{col}" opacity="{op}"/>'
                    f'<line x1="{f(ax+aw/2)}" y1="{f(ay+aw*0.2)}" x2="{f(ax+aw/2)}" y2="{f(ay+ah)}" stroke="{fill}" stroke-width="{f(1.3*scale)}"/>')
        n = 5
        aw = (w - tw * 1.8) / (n * 2.1)
        for i in range(n):
            ax = x + tw * 0.9 + aw * 0.55 + i * aw * 2.1
            g.append(arch(ax, hz - bh * 0.42, aw, bh * 0.36))
        for tx in (x, x + w - tw):
            for j in range(3):
                g.append(arch(tx + tw * 0.28, hz - th + th * (0.12 + j * 0.27), tw * 0.44, th * 0.17, op=0.75))
        g.append(arch(cx - aw * 0.9, hz - bh * 0.3, aw * 1.8, bh * 0.3, col=AMBER2, op=0.9))  # portal
        return "".join(g), tips

    def handle_cap(self, gap, bx, bw, h, margin):
        """Cap a building's height so it stays below the 45-degree handle of the logo at (cx, cy, R)."""
        if not gap:
            return h
        cx, cy, R = gap
        hx0, hy0 = cx + 0.69 * R, cy + 0.69 * R          # handle start (just outside the ring)
        hx1, hy1 = cx + 1.38 * R, cy + 1.38 * R          # handle end (centre of the cap)
        half = R * 0.07                                   # half the handle width plus the end cap
        if bx + bw < hx0 - half - margin or bx > hx1 + half + margin:
            return h
        # the handle's lower edge at the building's right side, clamped at the end cap
        y_edge = min((bx + bw) - cx + cy, hy1) + half
        return min(h, max(0, self.hz - (y_edge + margin)))

    def city(self, scale=1.0, cathedral_at=(), far_scale=0.55, clear=(), gap=None, gap_margin=30, landmarks=None, fill_h=(120, 380)):
        r = self.r
        W, hz = self.W, self.hz
        # far layer
        far, x = [], -80
        while x < W + 80:
            w = r.uniform(30, 70) * scale
            h = r.uniform(120, 360) * scale * far_scale / 0.55 * 0.8
            h = self.handle_cap(gap, x, w, h, gap_margin * 0.5)
            s, _ = self.skyscraper(x, w, h, scale * far_scale, far=True)
            far.append(s); x += w + r.uniform(4, 18)
        self.body.append(f'<g opacity="0.9" filter="url(#blur3)">{"".join(far)}</g>')
        # haze between layers
        self.body.append(f'<rect x="0" y="{f(hz-300*scale)}" width="{W}" height="{f(300*scale)}" fill="url(#hazeg)" opacity="0.8"/>')
        # near layer
        near, x = [], -60
        if landmarks:
            from landmarks import LANDMARKS
            occupied, lm_svg = [], []
            for (name, cx, Hh) in landmarks:
                svg, tips, lw = LANDMARKS[name](cx, hz, Hh, r)
                x0, x1 = (cx - 0.3 * Hh, cx + 0.62 * Hh) if name == "cologne" else (cx - lw / 2, cx + lw / 2)
                occupied.append((x0, x1)); lm_svg.append(svg); self.landmarks += tips
            while x < W + 60:
                w = r.uniform(36, 90) * scale
                hit = [o for o in occupied if x < o[1] and x + w > o[0]]
                if hit:
                    x = max(o[1] for o in hit) + r.uniform(4, 16); continue
                h = r.uniform(*fill_h) * scale
                for (zx0, zx1, zmax) in clear:
                    if x < zx1 and x + w > zx0:
                        h = min(h, zmax * r.uniform(0.6, 1.0))
                h = self.handle_cap(gap, x, w, h, gap_margin)
                sv, tip = self.skyscraper(x, w, h, scale)
                near.append(sv)
                x += w + r.uniform(5, 22)
            near += lm_svg
            self.body.append(f'<g>{"".join(near)}</g>')
            self.body.append(f'<rect x="0" y="{f(hz-2)}" width="{W}" height="4" fill="{CYAN}" opacity="0.35"/>')
            return
        cath = sorted(cathedral_at)
        while x < W + 60:
            if cath and x >= cath[0] - 10:
                cw = r.uniform(230, 290) * scale
                s, tips = self.cathedral(x, cw, scale)
                near.append(s); self.landmarks += [tips[-1]] + tips[:2]
                x += cw + r.uniform(14, 30); cath.pop(0); continue
            w = r.uniform(42, 110) * scale
            h = r.uniform(170, 640) * scale
            if r.random() < 0.18:
                h *= 1.25
            for (zx0, zx1, zmax) in clear:
                if x < zx1 and x + w > zx0:
                    h = min(h, zmax * r.uniform(0.6, 1.0))
            h = self.handle_cap(gap, x, w, h, gap_margin)
            s, tip = self.skyscraper(x, w, h, scale)
            near.append(s)
            if h > 560 * scale:
                self.landmarks.append(tip)
            x += w + r.uniform(5, 26)
        self.body.append(f'<g>{"".join(near)}</g>')
        self.body.append(f'<rect x="0" y="{f(hz-2)}" width="{W}" height="4" fill="{CYAN}" opacity="0.35"/>')

    def links_to_city(self, k):
        r = self.r
        edges = []
        lm = sorted(self.landmarks, key=lambda p: p[1])[:k]
        src = sorted(self.net_nodes, key=lambda p: -p[1])[: max(8, len(self.net_nodes) // 3)]
        g, beacons = [], []
        for (tx, ty) in lm:
            s = min(src, key=lambda p: abs(p[0] - tx) * 1.6 + abs(p[1] - ty))
            cx, cy = (s[0] + tx) / 2 + r.uniform(-60, 60), (s[1] + ty) / 2 - r.uniform(30, 120)
            g.append(f'<path d="M{f(s[0])},{f(s[1])} Q{f(cx)},{f(cy)} {f(tx)},{f(ty)}" fill="none" stroke="{CYAN2}" stroke-width="1.4" opacity="0.5" stroke-dasharray="7 9"/>')
            t = r.uniform(0.3, 0.75)
            px = (1 - t) ** 2 * s[0] + 2 * (1 - t) * t * cx + t ** 2 * tx
            py = (1 - t) ** 2 * s[1] + 2 * (1 - t) * t * cy + t ** 2 * ty
            g.append(f'<circle cx="{f(px)}" cy="{f(py)}" r="9" fill="{CYAN2}" opacity="0.25"/><circle cx="{f(px)}" cy="{f(py)}" r="3.2" fill="#FFFFFF"/>')
            beacons.append((tx, ty))
        self.body.append("".join(g))
        for (x, y) in beacons:
            self.node(x, y, 4.2, col="#FFFFFF", ring=CYAN2)

    # ----------------------------------------------------------------- water
    def water(self, scene_id):
        W, H, hz = self.W, self.H, self.hz
        self.defs.append(f'<linearGradient id="waterg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#0B1A3A"/><stop offset="1" stop-color="#02040A"/></linearGradient>'
                         f'<linearGradient id="reflfade" gradientUnits="userSpaceOnUse" x1="0" y1="{hz}" x2="0" y2="{H}"><stop offset="0" stop-color="#fff" stop-opacity="0.55"/><stop offset="0.7" stop-color="#fff" stop-opacity="0.08"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>'
                         f'<mask id="reflmask"><rect x="0" y="{hz}" width="{W}" height="{H-hz}" fill="url(#reflfade)"/></mask>')
        self.body.append(f'<rect x="0" y="{hz}" width="{W}" height="{H-hz}" fill="url(#waterg)"/>')
        self.body.append(f'<g mask="url(#reflmask)" filter="url(#blur4)"><use href="#{scene_id}" transform="translate(0,{2*hz}) scale(1,-1)"/></g>')
        # ripples
        rip, y = [], hz + 3
        r = self.r
        while y < H:
            rip.append(f'<rect x="0" y="{f(y)}" width="{W}" height="{f(r.uniform(1.5,4))}" fill="#02040A" opacity="{f(r.uniform(0.25,0.6))}"/>')
            y += r.uniform(5, 13)
        self.body.append("".join(rip))
        # glints
        gl = []
        for _ in range(int(W / 18)):
            x = r.uniform(0, W); y = hz + r.uniform(4, (H - hz) * 0.7)
            gl.append(f'<line x1="{f(x)}" y1="{f(y)}" x2="{f(x+r.uniform(8,40))}" y2="{f(y)}" stroke="{CYAN2}" stroke-width="1.2" opacity="{f(r.uniform(0.1,0.4))}"/>')
        self.body.append("".join(gl))

    # ------------------------------------------------------------------- svg
    def svg(self, extra=""):
        W, H = self.W, self.H
        filters = "".join(f'<filter id="blur{n}" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="{n}"/></filter>' for n in (3, 4, 16, 34, 60))
        filters += '<linearGradient id="hazeg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#1E5C8C" stop-opacity="0"/><stop offset="1" stop-color="#1E5C8C" stop-opacity="0.35"/></linearGradient>'
        return (f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{W}" height="{H}" viewBox="0 0 {W} {H}">'
                f'<defs>{filters}{"".join(self.defs)}</defs><rect width="{W}" height="{H}" fill="#03050C"/>'
                f'{"".join(self.body)}{extra}</svg>')


def build(W, H, hz, seed, q, curtains, cathedral_at, city_scale, stars, link_k, text="", clear=(), gap_margin=30, landmarks=None, fill_h=(120, 380)):
    s = Scene(W, H, hz, seed)
    s.sky(); s.stars(stars)
    # everything above the waterline lives in a group so the water can reflect it
    s.body.append('<g id="above">')
    layers = []
    for c in curtains:
        layers.append(s.curtain(**c))
    s.network(layers)
    s.logo(*q)
    # tie the logo into the net: outer nodes -> nearest network nodes
    edges = []
    for ln in s.logo_nodes:
        n = min(s.net_nodes, key=lambda p: (p[0] - ln[0]) ** 2 + (p[1] - ln[1]) ** 2)
        if math.hypot(n[0] - ln[0], n[1] - ln[1]) < q[2] * 1.3:
            edges.append((ln, n))
    s.draw_edges(edges, op=0.6, w=1.8, pulse_p=0.7)
    s.city(scale=city_scale, cathedral_at=cathedral_at, clear=clear, gap=(q if gap_margin else None), gap_margin=gap_margin or 0, landmarks=landmarks, fill_h=fill_h)
    s.links_to_city(link_k)
    s.body.append('</g>')
    s.water("above")
    return s.svg(text)


def cover_text(W):
    T = "Avenir Next Condensed, Futura, Helvetica Neue, Arial"
    B = "Avenir Next, Helvetica Neue, Arial"
    return f'''
<defs><linearGradient id="titleg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#FFFFFF"/><stop offset="1" stop-color="#A9EEFF"/></linearGradient></defs>
<g text-anchor="middle" style="font-family:{T}">
  <text x="{W/2}" y="150" fill="{CYAN2}" font-size="34" font-weight="600" letter-spacing="16" style="font-family:{B}">THE</text>
  <g filter="url(#blur16)" opacity="0.55" fill="{CYAN}">
    <text x="{W/2}" y="318" font-size="196" font-weight="700" letter-spacing="4">QUERYGRAPH</text>
    <text x="{W/2}" y="490" font-size="196" font-weight="700" letter-spacing="26">STACK</text>
  </g>
  <text x="{W/2}" y="318" fill="url(#titleg)" font-size="196" font-weight="700" letter-spacing="4">QUERYGRAPH</text>
  <text x="{W/2}" y="490" fill="url(#titleg)" font-size="196" font-weight="700" letter-spacing="26">STACK</text>
  <text x="{W/2}" y="578" fill="{GREEN}" font-size="33" font-weight="600" letter-spacing="7" style="font-family:{B}">THE DEFINITIVE GUIDE TO</text>
  <text x="{W/2}" y="626" fill="{GREEN}" font-size="33" font-weight="600" letter-spacing="7" style="font-family:{B}">THE GOVERNED SEMANTIC LAKEHOUSE</text>
  <rect x="{W/2-560}" y="2168" width="1120" height="150" rx="6" fill="#03050C" opacity="0.55"/>
  <text x="{W/2}" y="2232" fill="{WHITE}" font-size="54" font-weight="600" letter-spacing="7">ALEXY KHRABROV <tspan fill="{LILAC}" font-size="36" font-weight="500" letter-spacing="2" style="font-family:{B}">and</tspan> SLAVA TYKHONOV</text>
  <text x="{W/2}" y="2292" fill="{CYAN2}" font-size="22" font-weight="600" letter-spacing="9" style="font-family:{B}">FIRST PAIR PRESS</text>
</g>'''


if __name__ == "__main__":
    out = pathlib.Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
    # --- book cover 1600 x 2400 (6 x 9)
    W, H = 1600, 2400
    cover = build(
        W, H, hz=1960, seed=7, q=(800, 1110, 300),
        curtains=[
            dict(base=830, amp=65, wave=260, phase=0.4, thick=100, ray_len=420, hue=GREEN, top_hue=VIOLET),
            dict(base=1040, amp=110, wave=330, phase=2.3, thick=130, ray_len=480, hue=CYAN, top_hue=MAGENTA),
            dict(base=1330, amp=80, wave=210, phase=4.1, thick=95, ray_len=380, hue=GREEN, top_hue=VIOLET),
        ],
        cathedral_at=(), city_scale=1.0, stars=420, link_k=10, text=cover_text(W), gap_margin=34, fill_h=(110, 340),
        landmarks=[("eiffel", 110, 500), ("cologne", 340, 330), ("empire", 590, 560), ("bank_of_china", 705, 450),
                   ("ping_an", 805, 680), ("oriental_pearl", 925, 560), ("shanghai_tower", 1300, 620), ("jin_mao", 1395, 480),
                   ("spasskaya", 1470, 300), ("fernsehturm", 1560, 470)])
    (out / "querygraph-stack-cover.svg").write_text(cover, encoding="utf-8")
    # --- headboard 2400 x 900, no text
    W, H = 2400, 900
    head = build(
        W, H, hz=735, seed=11, q=(1200, 372, 215),
        curtains=[
            dict(base=150, amp=45, wave=380, phase=0.9, thick=70, ray_len=280, hue=GREEN, top_hue=VIOLET),
            dict(base=290, amp=65, wave=470, phase=2.8, thick=85, ray_len=320, hue=CYAN, top_hue=MAGENTA),
            dict(base=450, amp=50, wave=300, phase=4.6, thick=65, ray_len=240, hue=GREEN, top_hue=VIOLET),
        ],
        cathedral_at=(), city_scale=0.72, stars=520, link_k=12, gap_margin=26, fill_h=(90, 300),
        landmarks=[("cologne", 240, 210), ("oriental_pearl", 470, 360), ("jin_mao", 640, 300), ("ping_an", 1000, 300),
                   ("shanghai_tower", 1250, 330), ("fernsehturm", 1640, 330), ("spasskaya", 1740, 200), ("empire", 1900, 350),
                   ("bank_of_china", 2060, 290), ("eiffel", 2250, 280)])
    (out / "querygraph-stack-headboard.svg").write_text(head, encoding="utf-8")
    print("wrote", out)
