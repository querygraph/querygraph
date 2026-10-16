import sys, pathlib
from covers import build, cover_text, GREEN, CYAN, VIOLET, MAGENTA
out = pathlib.Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
W, H = 1600, 2400
(out / "cover-v1.svg").write_text(build(W, H, hz=1960, seed=7, q=(800, 1110, 300), curtains=[
    dict(base=720, amp=85, wave=260, phase=0.4, thick=110, ray_len=420, hue=GREEN, top_hue=VIOLET),
    dict(base=960, amp=120, wave=330, phase=2.3, thick=130, ray_len=480, hue=CYAN, top_hue=MAGENTA),
    dict(base=1290, amp=90, wave=210, phase=4.1, thick=95, ray_len=380, hue=GREEN, top_hue=VIOLET)],
    cathedral_at=(130, 1040), city_scale=1.0, stars=420, link_k=9, text=cover_text(W), gap_margin=None), encoding="utf-8")
W, H = 2400, 900
(out / "headboard-v1.svg").write_text(build(W, H, hz=735, seed=11, q=(1200, 372, 215), curtains=[
    dict(base=170, amp=55, wave=380, phase=0.9, thick=80, ray_len=300, hue=GREEN, top_hue=VIOLET),
    dict(base=330, amp=75, wave=470, phase=2.8, thick=95, ray_len=340, hue=CYAN, top_hue=MAGENTA),
    dict(base=520, amp=55, wave=300, phase=4.6, thick=70, ray_len=260, hue=GREEN, top_hue=VIOLET)],
    cathedral_at=(300, 1480, 2010), city_scale=0.72, stars=520, link_k=12, gap_margin=None), encoding="utf-8")
print("v1 svgs written")
