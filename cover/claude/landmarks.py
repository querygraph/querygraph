"""Landmark silhouettes for the QueryGraph skyline.

Each function takes (cx, hz, H, r): centre x, horizon y, total height, rng.
It returns (svg, tips, width) where tips are (x, y) points the aurora network
can link to and width is the footprint used to keep filler buildings clear.
Coordinates: y grows downward, buildings stand on hz and rise to hz - H.
"""
import math

DARK = "#05070F"; CYAN = "#35D9EE"; CYAN2 = "#7FEFFB"; AMBER = "#FFC46B"; AMBER2 = "#FFE2A8"
LILAC = "#B48BFF"; MAGENTA = "#E07CFF"; RED = "#FF5A4E"; WHITE = "#EFFFFF"; BRICK = "#1A0B10"


def f(x):
    return f"{x:.1f}".rstrip("0").rstrip(".")


def P(pts):
    return " ".join(f"{f(x)},{f(y)}" for x, y in pts)


def poly(pts, fill=DARK, op=1):
    return f'<polygon points="{P(pts)}" fill="{fill}" opacity="{op}"/>'


def rect(x, y, w, h, fill=DARK):
    return f'<rect x="{f(x)}" y="{f(y)}" width="{f(w)}" height="{f(h)}" fill="{fill}"/>'


def rim(pts, col=CYAN2, op=0.45, w=1.2, close=False):
    tag = "polygon" if close else "polyline"
    return f'<{tag} points="{P(pts)}" fill="none" stroke="{col}" stroke-width="{w}" opacity="{op}" stroke-linejoin="round"/>'


def line(x1, y1, x2, y2, col=CYAN2, op=0.35, w=1.2):
    return f'<line x1="{f(x1)}" y1="{f(y1)}" x2="{f(x2)}" y2="{f(y2)}" stroke="{col}" stroke-width="{w}" opacity="{op}" stroke-linecap="round"/>'


def dot(x, y, rad, col, op=0.9, glow=3.0):
    return (f'<circle cx="{f(x)}" cy="{f(y)}" r="{f(rad*glow)}" fill="{col}" opacity="{f(op*0.2)}"/>'
            f'<circle cx="{f(x)}" cy="{f(y)}" r="{f(rad)}" fill="{col}" opacity="{op}"/>')


def windows(r, x, y, w, h, cw, ch, p=0.5, cols_fill=0.5, cols=("#CFE9FF", "#CFE9FF", CYAN2, "#FFF1C9")):
    out = []
    ncol = max(1, int(w // cw)); nrow = max(1, int(h // ch))
    ox = x + (w - ncol * cw) / 2 + cw * (1 - cols_fill) / 2
    for i in range(ncol):
        for j in range(nrow):
            if r.random() < p:
                out.append(f'<rect x="{f(ox+i*cw)}" y="{f(y+ch*0.25+j*ch)}" width="{f(cw*cols_fill)}" height="{f(ch*0.5)}" fill="{r.choice(cols)}" opacity="{f(r.uniform(0.25,0.85))}"/>')
    return "".join(out)


def strips(r, x, y, w, h, n, col=CYAN2, p=0.7):
    """Vertical lit strips (curtain-wall look)."""
    out = []
    for i in range(n):
        if r.random() < p:
            sx = x + w * (i + 0.5) / n
            seg_top = y + h * r.uniform(0.0, 0.3)
            out.append(f'<line x1="{f(sx)}" y1="{f(seg_top)}" x2="{f(sx)}" y2="{f(y+h)}" stroke="{col}" stroke-width="1.2" opacity="{f(r.uniform(0.2,0.55))}"/>')
    return "".join(out)


# ---------------------------------------------------------------- New York
def empire(cx, hz, H, r):
    w = 0.30 * H
    Y = lambda t: hz - H * t
    tiers = [(1.0, 0.0, 0.14), (0.66, 0.14, 0.62), (0.52, 0.62, 0.71), (0.36, 0.71, 0.79), (0.15, 0.79, 0.90)]
    g = []
    for (fw, t0, t1) in tiers:
        tw = w * fw
        g.append(rect(cx - tw / 2, Y(t1), tw, H * (t1 - t0)))
    g.append(line(cx, Y(0.90), cx, Y(1.0), col="#0B1430", op=1, w=3))
    for (fw, t0, t1) in tiers[:4]:
        tw = w * fw
        g.append(rim([(cx - tw / 2, Y(t1) + H * (t1 - t0)), (cx - tw / 2, Y(t1)), (cx + tw / 2, Y(t1)), (cx + tw / 2, Y(t1) + H * (t1 - t0))], op=0.4))
        g.append(windows(r, cx - tw / 2 + 4, Y(t1) + 4, tw - 8, H * (t1 - t0) - 6, 9, 11, p=0.55, cols_fill=0.45))
    # mooring mast lit, beacon
    g.append(rect(cx - w * 0.075, Y(0.90), w * 0.15, H * 0.11, fill=DARK))
    g.append(line(cx - w * 0.075, Y(0.79), cx - w * 0.075, Y(0.90), col=AMBER, op=0.6, w=1.5))
    g.append(line(cx + w * 0.075, Y(0.79), cx + w * 0.075, Y(0.90), col=AMBER, op=0.6, w=1.5))
    g.append(dot(cx, Y(1.0), 3, RED))
    return "".join(g), [(cx, Y(1.0))], w


# ------------------------------------------------------------------- Paris
def eiffel(cx, hz, H, r):
    Y = lambda t: hz - H * t
    X = lambda t: cx + H * t
    g = []
    # legs (solid lattice bands) with the arch between them
    for s in (-1, 1):
        g.append(f'<path d="M{f(X(0.26*s))},{f(Y(0))} Q{f(X(0.23*s))},{f(Y(0.12))} {f(X(0.15*s))},{f(Y(0.19))} L{f(cx)},{f(Y(0.19))} L{f(cx)},{f(Y(0.15))} Q{f(X(0.14*s))},{f(Y(0.15))} {f(X(0.16*s))},{f(Y(0))} Z" fill="{DARK}"/>')
    # first platform
    g.append(rect(X(-0.175), Y(0.215), H * 0.35, H * 0.025))
    # second stage (concave) + platform
    g.append(f'<path d="M{f(X(-0.15))},{f(Y(0.215))} Q{f(X(-0.09))},{f(Y(0.33))} {f(X(-0.07))},{f(Y(0.38))} L{f(X(0.07))},{f(Y(0.38))} Q{f(X(0.09))},{f(Y(0.33))} {f(X(0.15))},{f(Y(0.215))} Z" fill="{DARK}"/>')
    g.append(rect(X(-0.09), Y(0.40), H * 0.18, H * 0.02))
    # upper tapering shaft + top platform + antenna
    g.append(f'<path d="M{f(X(-0.065))},{f(Y(0.40))} Q{f(X(-0.03))},{f(Y(0.62))} {f(X(-0.012))},{f(Y(0.93))} L{f(X(0.012))},{f(Y(0.93))} Q{f(X(0.03))},{f(Y(0.62))} {f(X(0.065))},{f(Y(0.40))} Z" fill="{DARK}"/>')
    g.append(rect(X(-0.03), Y(0.95), H * 0.06, H * 0.02))
    g.append(line(cx, Y(0.95), cx, Y(1.0), col="#0B1430", op=1, w=2.5))
    # lattice hints
    lat = []
    for t in [0.05, 0.1, 0.15, 0.26, 0.31, 0.36]:
        # half width at height t (approx of the outer profile)
        hw = 0.26 - 0.11 * (t / 0.19) if t < 0.19 else 0.15 - 0.08 * ((t - 0.215) / 0.165)
        lat.append(line(X(-hw), Y(t), X(hw), Y(t), op=0.22, w=1))
    for t in [0.45, 0.55, 0.65, 0.75, 0.85]:
        hw = 0.065 - 0.053 * ((t - 0.40) / 0.53)
        lat.append(line(X(-hw), Y(t), X(hw), Y(t), op=0.22, w=1))
    for s in (-1, 1):
        lat.append(line(X(0.26 * s), Y(0), X(0.15 * s), Y(0.19), op=0.3, w=1))
        lat.append(line(X(0.16 * s), Y(0.02), X(0.05 * s), Y(0.15), op=0.2, w=1))
        lat.append(line(X(0.15 * s), Y(0.215), X(0.07 * s), Y(0.38), op=0.3, w=1))
    g.append("".join(lat))
    # golden illumination: beads along the outer profile and platforms
    beads = []
    for t in [i / 40 for i in range(1, 38)]:
        if t < 0.19:
            hw = 0.26 - 0.11 * (t / 0.19)
        elif t < 0.215:
            hw = 0.175
        elif t < 0.38:
            hw = 0.15 - 0.08 * ((t - 0.215) / 0.165)
        elif t < 0.40:
            hw = 0.09
        else:
            hw = 0.065 - 0.053 * ((t - 0.40) / 0.53)
        for s in (-1, 1):
            beads.append(dot(X(hw * s), Y(t), 1.5, AMBER, op=r.uniform(0.5, 0.9), glow=3.5))
    for xx in [i / 14 for i in range(-7, 8)]:
        beads.append(dot(X(0.175 * xx), Y(0.215), 1.3, AMBER2, op=0.8, glow=3))
    g.append("".join(beads))
    g.append(dot(cx, Y(1.0), 2.6, WHITE))
    return "".join(g), [(cx, Y(1.0))], 0.54 * H


# ---------------------------------------------------------------- Shanghai
def oriental_pearl(cx, hz, H, r):
    Y = lambda t: hz - H * t
    g = []
    # tripod legs + small base pearls
    for s in (-1, 1):
        g.append(line(cx + s * 0.13 * H, hz, cx + s * 0.03 * H, Y(0.32), col=DARK, op=1, w=H * 0.025))
        g.append(f'<circle cx="{f(cx + s*0.09*H)}" cy="{f(Y(0.05))}" r="{f(H*0.035)}" fill="{DARK}"/>')
        g.append(f'<circle cx="{f(cx + s*0.09*H)}" cy="{f(Y(0.05))}" r="{f(H*0.035)}" fill="none" stroke="{MAGENTA}" stroke-width="1.2" opacity="0.5"/>')
    g.append(rect(cx - 0.03 * H, Y(0.36), 0.06 * H, H * 0.36))
    # lower pearl
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.38))}" r="{f(H*0.105)}" fill="{DARK}"/>')
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.38))}" r="{f(H*0.105)}" fill="none" stroke="{MAGENTA}" stroke-width="1.6" opacity="0.6"/>')
    g.append(f'<ellipse cx="{f(cx)}" cy="{f(Y(0.38))}" rx="{f(H*0.1)}" ry="{f(H*0.022)}" fill="{MAGENTA}" opacity="0.35"/>')
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.38))}" r="{f(H*0.16)}" fill="{MAGENTA}" opacity="0.07"/>')
    # column with small pearls, upper pearl, top pearl, antenna
    g.append(rect(cx - 0.022 * H, Y(0.72), 0.044 * H, H * 0.36))
    for t in (0.52, 0.58, 0.64):
        g.append(f'<circle cx="{f(cx)}" cy="{f(Y(t))}" r="{f(H*0.022)}" fill="{DARK}" stroke="{LILAC}" stroke-width="1" opacity="0.9"/>')
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.74))}" r="{f(H*0.062)}" fill="{DARK}"/>')
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.74))}" r="{f(H*0.062)}" fill="none" stroke="{MAGENTA}" stroke-width="1.5" opacity="0.6"/>')
    g.append(f'<ellipse cx="{f(cx)}" cy="{f(Y(0.74))}" rx="{f(H*0.058)}" ry="{f(H*0.014)}" fill="{LILAC}" opacity="0.4"/>')
    g.append(rect(cx - 0.012 * H, Y(0.86), 0.024 * H, H * 0.14))
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.87))}" r="{f(H*0.028)}" fill="{DARK}" stroke="{MAGENTA}" stroke-width="1.2" opacity="0.9"/>')
    g.append(line(cx, Y(0.89), cx, Y(1.0), col="#0B1430", op=1, w=2.5))
    g.append(dot(cx, Y(1.0), 2.6, RED))
    return "".join(g), [(cx, Y(1.0))], 0.3 * H


def shanghai_tower(cx, hz, H, r):
    Y = lambda t: hz - H * t
    X = lambda t: cx + H * t
    pts = [(X(-0.10), hz), (X(-0.088), Y(0.90)), (X(-0.03), Y(1.0)), (X(0.05), Y(0.955)), (X(0.078), Y(0.86)), (X(0.10), hz)]
    g = [poly(pts)]
    g.append(rim(pts[:5], op=0.45))
    # the twist: sweeping diagonals
    for k in range(4):
        y0 = Y(0.08 + k * 0.22); y1 = Y(0.3 + k * 0.22)
        g.append(f'<path d="M{f(X(-0.095))},{f(y0)} Q{f(cx)},{f(y0 - H*0.06)} {f(X(0.085))},{f(y1)}" fill="none" stroke="{CYAN2}" stroke-width="1.8" opacity="0.6"/>')
    g.append(windows(r, X(-0.085), Y(0.86) + 6, 0.165 * H, H * 0.84, 10, 12, p=0.4, cols_fill=0.45, cols=(CYAN2, "#CFE9FF", "#CFE9FF", "#FFF1C9")))
    g.append(strips(r, X(-0.085), Y(0.86), 0.165 * H, H * 0.84, 12, p=0.8))
    g.append(dot(X(-0.03), Y(1.0), 2.4, RED))
    return "".join(g), [(X(-0.03), Y(1.0))], 0.22 * H


def jin_mao(cx, hz, H, r):
    Y = lambda t: hz - H * t
    g = []
    hw = 0.11 * H
    t = 0.0
    tops = []
    for seg in (0.48, 0.13, 0.1, 0.07, 0.05, 0.04, 0.03):
        g.append(rect(cx - hw, Y(t + seg), 2 * hw, H * seg))
        g.append(line(cx - hw, Y(t + seg), cx + hw, Y(t + seg), op=0.4, w=1))
        tops.append((hw, t + seg))
        t += seg; hw *= 0.85
    g.append(poly([(cx - hw * 0.9, Y(t)), (cx, Y(1.0)), (cx + hw * 0.9, Y(t))]))
    g.append(strips(r, cx - 0.1 * H, Y(0.48), 0.2 * H, H * 0.48, 10, p=0.7))
    g.append(strips(r, cx - 0.08 * H, Y(0.78), 0.16 * H, H * 0.3, 6, p=0.7))
    g.append(dot(cx, Y(1.0), 2.4, RED))
    return "".join(g), [(cx, Y(1.0))], 0.24 * H


# ---------------------------------------------------------------- Shenzhen
def ping_an(cx, hz, H, r):
    Y = lambda t: hz - H * t
    hw = 0.085 * H
    g = [rect(cx - hw, Y(0.86), 2 * hw, H * 0.86)]
    g.append(poly([(cx - hw, Y(0.86)), (cx - hw * 0.25, Y(0.975)), (cx, Y(1.0)), (cx + hw * 0.25, Y(0.975)), (cx + hw, Y(0.86))]))
    g.append(rim([(cx - hw, hz), (cx - hw, Y(0.86)), (cx - hw * 0.25, Y(0.975)), (cx, Y(1.0)), (cx + hw * 0.25, Y(0.975)), (cx + hw, Y(0.86)), (cx + hw, hz)], op=0.5))
    # stainless fins
    for i in range(1, 8):
        sx = cx - hw + 2 * hw * i / 8
        g.append(line(sx, Y(0.02), sx, Y(0.86 + 0.11 * (1 - abs(i - 4) / 4)), op=0.3, w=1))
    g.append(windows(r, cx - hw + 3, Y(0.86) + 4, 2 * hw - 6, H * 0.85, 8, 10, p=0.45, cols_fill=0.4, cols=(CYAN2, "#CFE9FF", "#CFE9FF")))
    g.append(dot(cx, Y(1.0), 2.6, RED))
    return "".join(g), [(cx, Y(1.0))], 0.19 * H


# --------------------------------------------------------------- Hong Kong
def bank_of_china(cx, hz, H, r):
    Y = lambda t: hz - H * t
    w = 0.11 * H
    body = [(cx - w, hz), (cx - w, Y(0.44)), (cx, Y(0.58)), (cx, Y(0.58)), (cx + w * 0.45, Y(0.72)), (cx + w * 0.45, Y(0.72)), (cx + w, Y(0.88)), (cx + w, hz)]
    g = [poly(body)]
    g.append(rim([(cx - w, Y(0.44)), (cx, Y(0.58)), (cx, Y(0.58))], op=0.5))
    g.append(rim([(cx, Y(0.58)), (cx + w * 0.45, Y(0.72))], op=0.5))
    g.append(rim([(cx + w * 0.45, Y(0.72)), (cx + w, Y(0.88))], op=0.5))
    g.append(line(cx, Y(0.58), cx, hz, op=0.3))
    g.append(line(cx + w * 0.45, Y(0.72), cx + w * 0.45, hz, op=0.3))
    # the X bracing
    cell = H * 0.145
    for k in range(6):
        y0 = hz - k * cell; y1 = y0 - cell
        for (x0, x1, tcap) in ((cx - w, cx, Y(0.44)), (cx, cx + w * 0.45, Y(0.58)), (cx + w * 0.45, cx + w, Y(0.72))):
            if y1 < tcap - cell * 0.3:
                continue
            g.append(line(x0, y0, x1, y1, op=0.32, w=1.3))
            g.append(line(x0, y1, x1, y0, op=0.32, w=1.3))
    g.append(line(cx + w * 0.55, Y(0.88), cx + w * 0.55, Y(1.0), col="#0B1430", op=1, w=2.5))
    g.append(line(cx + w * 0.85, Y(0.88), cx + w * 0.85, Y(0.97), col="#0B1430", op=1, w=2.5))
    g.append(dot(cx + w * 0.55, Y(1.0), 2.4, RED)); g.append(dot(cx + w * 0.85, Y(0.97), 2, RED))
    g.append(windows(r, cx - w + 3, Y(0.44) + 6, w * 2 - 6, H * 0.43, 9, 11, p=0.35, cols_fill=0.4))
    return "".join(g), [(cx + w * 0.55, Y(1.0))], 0.24 * H


# ------------------------------------------------------------------ Moscow
def spasskaya(cx, hz, H, r):
    Y = lambda t: hz - H * t
    w = 0.30 * H
    g = [rect(cx - w / 2, Y(0.44), w, H * 0.44, fill=BRICK)]
    # corner turrets on the base
    for s in (-1, 1):
        tx = cx + s * w * 0.42
        g.append(poly([(tx - w * 0.06, Y(0.44)), (tx, Y(0.52)), (tx + w * 0.06, Y(0.44))], fill=BRICK))
    # clock tier
    g.append(rect(cx - w * 0.4, Y(0.62), w * 0.8, H * 0.18, fill=BRICK))
    cr = w * 0.13
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.53))}" r="{f(cr*1.9)}" fill="{AMBER}" opacity="0.12"/>')
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.53))}" r="{f(cr)}" fill="{AMBER2}" opacity="0.95"/>')
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.53))}" r="{f(cr*0.8)}" fill="none" stroke="{BRICK}" stroke-width="1.2"/>')
    g.append(line(cx, Y(0.53), cx, Y(0.53) - cr * 0.65, col=BRICK, op=1, w=2))
    g.append(line(cx, Y(0.53), cx + cr * 0.5, Y(0.53) + cr * 0.25, col=BRICK, op=1, w=2))
    # arcaded belfry tier
    g.append(rect(cx - w * 0.28, Y(0.70), w * 0.56, H * 0.08, fill=BRICK))
    for i in range(3):
        ax = cx - w * 0.2 + i * w * 0.2
        g.append(f'<path d="M{f(ax-w*0.045)},{f(Y(0.62))} L{f(ax-w*0.045)},{f(Y(0.665))} Q{f(ax)},{f(Y(0.70))} {f(ax+w*0.045)},{f(Y(0.665))} L{f(ax+w*0.045)},{f(Y(0.62))} Z" fill="{AMBER}" opacity="0.7"/>')
    # tented spire, star
    g.append(poly([(cx - w * 0.3, Y(0.70)), (cx, Y(0.93)), (cx + w * 0.3, Y(0.70))], fill="#0B1F1C"))
    g.append(rim([(cx - w * 0.3, Y(0.70)), (cx, Y(0.93)), (cx + w * 0.3, Y(0.70))], col="#4FD1A8", op=0.5))
    g.append(line(cx, Y(0.93), cx, Y(0.955), col="#0B1430", op=1, w=2))
    sr = H * 0.045
    star = []
    for k in range(10):
        a = math.radians(-90 + k * 36); rr = sr if k % 2 == 0 else sr * 0.42
        star.append((cx + math.cos(a) * rr, Y(0.955) - sr + math.sin(a) * rr))
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.955)-sr)}" r="{f(sr*2.4)}" fill="{RED}" opacity="0.18"/>')
    g.append(poly(star, fill=RED))
    g.append(rim([(cx - w / 2, hz), (cx - w / 2, Y(0.44)), (cx + w / 2, Y(0.44)), (cx + w / 2, hz)], col=AMBER, op=0.35))
    g.append(windows(r, cx - w * 0.4, Y(0.4), w * 0.8, H * 0.36, 14, 16, p=0.3, cols_fill=0.4, cols=(AMBER, AMBER2)))
    return "".join(g), [(cx, Y(0.955) - 2 * sr)], w


# ------------------------------------------------------------------ Berlin
def fernsehturm(cx, hz, H, r):
    Y = lambda t: hz - H * t
    g = [rect(cx - 0.07 * H, Y(0.06), 0.14 * H, H * 0.06)]
    g.append(poly([(cx - 0.034 * H, hz), (cx - 0.02 * H, Y(0.62)), (cx + 0.02 * H, Y(0.62)), (cx + 0.034 * H, hz)]))
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.69))}" r="{f(H*0.075)}" fill="{DARK}"/>')
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.69))}" r="{f(H*0.075)}" fill="none" stroke="{CYAN2}" stroke-width="1.4" opacity="0.55"/>')
    g.append(f'<circle cx="{f(cx)}" cy="{f(Y(0.69))}" r="{f(H*0.12)}" fill="{CYAN}" opacity="0.07"/>')
    g.append(f'<ellipse cx="{f(cx)}" cy="{f(Y(0.69))}" rx="{f(H*0.072)}" ry="{f(H*0.016)}" fill="{CYAN2}" opacity="0.45"/>')
    g.append(f'<ellipse cx="{f(cx)}" cy="{f(Y(0.665))}" rx="{f(H*0.074)}" ry="{f(H*0.01)}" fill="{WHITE}" opacity="0.25"/>')
    g.append(rect(cx - 0.012 * H, Y(0.87), 0.024 * H, H * 0.12))
    g.append(line(cx, Y(0.87), cx, Y(1.0), col="#0B1430", op=1, w=3))
    for t in (0.9, 0.95):
        g.append(line(cx, Y(t), cx, Y(t + 0.025), col=RED, op=0.8, w=3))
    g.append(line(cx - 0.02 * H, Y(0.3), cx - 0.02 * H, Y(0.62), col=CYAN2, op=0.25, w=1))
    g.append(dot(cx, Y(1.0), 2.6, RED))
    return "".join(g), [(cx, Y(1.0))], 0.16 * H


# ----------------------------------------------------------------- Cologne
def cologne(cx, hz, H, r):
    """Cologne Cathedral: twin openwork spires dominate, long nave between. H is spire height."""
    Y = lambda t: hz - H * t
    tw = 0.19 * H          # tower width
    gap = 0.11 * H         # between towers
    nave_h = 0.36 * H
    gable = 0.12 * H
    g = []
    left = cx - gap / 2 - tw
    towers = [left, cx + gap / 2]
    # nave + transept behind (long low body running off to the right)
    g.append(rect(cx - gap / 2 - tw * 0.1, Y(0) - nave_h, gap + tw * 0.2, nave_h))
    g.append(rect(cx + gap / 2 + tw * 0.6, Y(0) - nave_h * 0.92, tw * 2.3, nave_h * 0.92))
    g.append(poly([(cx - gap / 2, Y(0) - nave_h), (cx, Y(0) - nave_h - gable), (cx + gap / 2, Y(0) - nave_h)]))
    # crossing fleche
    g.append(poly([(cx + gap / 2 + tw * 1.6, Y(0) - nave_h * 0.92), (cx + gap / 2 + tw * 1.75, Y(0.62)), (cx + gap / 2 + tw * 1.9, Y(0) - nave_h * 0.92)]))
    # buttress pinnacles along the nave
    for i in range(5):
        px = cx + gap / 2 + tw * (0.75 + i * 0.4)
        g.append(poly([(px - tw * 0.05, Y(0) - nave_h * 0.92), (px, Y(0) - nave_h * 1.12), (px + tw * 0.05, Y(0) - nave_h * 0.92)]))
    tips = []
    for tx in towers:
        # shaft to 0.66, openwork spire to 1.0
        g.append(rect(tx, Y(0.66), tw, H * 0.66))
        g.append(poly([(tx - tw * 0.05, Y(0.66)), (tx + tw / 2, Y(1.0)), (tx + tw * 1.05, Y(0.66))]))
        for px in (tx + tw * 0.08, tx + tw * 0.92):
            g.append(poly([(px - tw * 0.08, Y(0.66)), (px, Y(0.78)), (px + tw * 0.08, Y(0.66))]))
        # tracery on the spire and tall lancets on the shaft
        for k in range(1, 6):
            t = 0.66 + k * 0.06
            hw = tw * 0.55 * (1 - (t - 0.66) / 0.34)
            g.append(line(tx + tw / 2 - hw, Y(t), tx + tw / 2 + hw, Y(t), col=CYAN2, op=0.3, w=1))
        g.append(rim([(tx - tw * 0.05, Y(0.66)), (tx + tw / 2, Y(1.0)), (tx + tw * 1.05, Y(0.66))], op=0.5))
        g.append(line(tx + tw / 2, Y(0.70), tx + tw / 2, Y(0.98), col=CYAN2, op=0.25, w=1))
        for (y0, y1) in ((0.08, 0.26), (0.32, 0.5), (0.54, 0.64)):
            for wx in (tx + tw * 0.3, tx + tw * 0.7):
                aw = tw * 0.16; ay = Y(y1); ah = H * (y1 - y0)
                g.append(f'<path d="M{f(wx-aw/2)},{f(ay+ah)} L{f(wx-aw/2)},{f(ay+aw)} Q{f(wx)},{f(ay)} {f(wx+aw/2)},{f(ay+aw)} L{f(wx+aw/2)},{f(ay+ah)} Z" fill="{AMBER}" opacity="{0.75 if y0 > 0.5 else 0.55}"/>')
                g.append(line(wx, ay + aw * 0.6, wx, ay + ah, col=DARK, op=1, w=1.2))
        tips.append((tx + tw / 2, Y(1.0)))
    # great west window between the towers + portal
    wx, wy, ww, wh = cx, Y(0) - nave_h * 0.55, gap * 0.5, nave_h * 0.42
    g.append(f'<path d="M{f(wx-ww/2)},{f(wy+wh)} L{f(wx-ww/2)},{f(wy+ww*0.6)} Q{f(wx)},{f(wy-ww*0.1)} {f(wx+ww/2)},{f(wy+ww*0.6)} L{f(wx+ww/2)},{f(wy+wh)} Z" fill="{AMBER}" opacity="0.8"/>')
    g.append(line(wx, wy + ww * 0.3, wx, wy + wh, col=DARK, op=1, w=1.5))
    g.append(f'<circle cx="{f(wx)}" cy="{f(wy+ww*0.6)}" r="{f(ww*0.22)}" fill="none" stroke="{DARK}" stroke-width="1.2"/>')
    g.append(f'<path d="M{f(wx-ww*0.3)},{f(hz)} L{f(wx-ww*0.3)},{f(hz-nave_h*0.18)} Q{f(wx)},{f(hz-nave_h*0.3)} {f(wx+ww*0.3)},{f(hz-nave_h*0.18)} L{f(wx+ww*0.3)},{f(hz)} Z" fill="{AMBER2}" opacity="0.85"/>')
    g.append(f'<ellipse cx="{f(cx)}" cy="{f(hz-nave_h*0.5)}" rx="{f(tw*2.2)}" ry="{f(nave_h*0.8)}" fill="{AMBER}" opacity="0.08"/>')
    total_w = 2 * tw + gap + tw * 2.3
    return "".join(g), tips, total_w


LANDMARKS = {
    "empire": empire, "eiffel": eiffel, "oriental_pearl": oriental_pearl, "shanghai_tower": shanghai_tower,
    "jin_mao": jin_mao, "ping_an": ping_an, "bank_of_china": bank_of_china, "spasskaya": spasskaya,
    "fernsehturm": fernsehturm, "cologne": cologne,
}
