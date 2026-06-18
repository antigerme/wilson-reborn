# SPDX-License-Identifier: GPL-3.0-or-later
"""Minimal but thorough NE (Win16) parser for the original SCRANTIC.EXE — segments, entry
table, imports and per-segment relocations resolved to targets. Foundation for the
disassembler (``disasm.py``).

Paths are taken from the environment so nothing copyright lives in the repo:

    SCRANTIC_EXE    path to the original SCRANTIC.EXE   (default: ./SCRANTIC.EXE)
    WINE_SPECS_DIR  dir with Wine .spec files for the   (default: ./specs)
                    Win16 modules (gdi/user/kernel/mmsystem) — used to turn
                    imported ordinals into API names. Optional but recommended.

Run standalone to print a summary (segments, modules, distinct imported API calls and the
entry table). See ``README.md`` for how to obtain the inputs.
"""
import struct, re, os

PATH = os.environ.get("SCRANTIC_EXE", "SCRANTIC.EXE")
SPECS_DIR = os.environ.get("WINE_SPECS_DIR", "specs")
D = open(PATH, "rb").read()

# Win16 ordinal->name maps from Wine .spec files (authoritative).
ORD = {}
for _mod, _fn in {"GDI": "gdi", "USER": "user", "KERNEL": "kernel", "MMSYSTEM": "mmsystem"}.items():
    m = {}
    _p = os.path.join(SPECS_DIR, f"{_fn}.spec")
    if os.path.exists(_p):
        for _line in open(_p):
            _o = re.match(r"\s*(\d+)\s+\w", _line)
            if not _o:
                continue
            _nm = re.search(r"([A-Za-z_]\w*)\s*\(", _line)
            m[int(_o.group(1))] = _nm.group(1) if _nm else _line.split()[-1]
    ORD[_mod] = m


def apiname(mod, ordn):
    return f"{mod}.{ORD.get(mod, {}).get(ordn, ordn)}"


def u8(o): return D[o]
def u16(o): return D[o] | (D[o + 1] << 8)
def u16b(b, o): return b[o] | (b[o + 1] << 8)


NE = u16(0x3C) | (u16(0x3E) << 16)
assert D[NE:NE + 2] == b"NE", "not NE"

ENTTAB = NE + u16(NE + 0x04)
CBENT = u16(NE + 0x06)
CSEG = u16(NE + 0x1C)
CMOD = u16(NE + 0x1E)
SEGTAB = NE + u16(NE + 0x22)
RESTAB = NE + u16(NE + 0x26)
MODTAB = NE + u16(NE + 0x28)
IMPTAB = NE + u16(NE + 0x2A)
ALIGN = u16(NE + 0x32) or 9


def pstr(o):
    n = D[o]
    return D[o + 1:o + 1 + n].decode("latin1", "replace")


def modname(idx):  # 1-based into module ref table
    off = u16(MODTAB + (idx - 1) * 2)
    return pstr(IMPTAB + off)


def impname(noff):
    return pstr(IMPTAB + noff)


class Seg:
    def __init__(self, i):
        o = SEGTAB + i * 8
        self.num = i + 1
        self.file_off = u16(o) << ALIGN
        self.length = u16(o + 2) or 0x10000
        self.flags = u16(o + 4)
        self.minalloc = u16(o + 6) or 0x10000
        self.is_data = bool(self.flags & 1)
        self.has_reloc = bool(self.flags & 0x0100)
        self.data = D[self.file_off:self.file_off + self.length]
        self.relocs = {}     # site_offset_in_seg -> target string
        self.reloc_kind = {} # site_offset -> addr_type
        if self.has_reloc:
            self._parse_relocs()

    def _parse_relocs(self):
        base = self.file_off + self.length
        n = u16(base)
        rp = base + 2
        for _ in range(n):
            rec = D[rp:rp + 8]; rp += 8
            addr_type = rec[0]; rel_type = rec[1]
            src = u16b(rec, 2)
            t = rel_type & 3
            if t == 0:  # INTERNALREF
                tseg = rec[4]
                if tseg == 0xFF:
                    target = f"ENTRY#{u16b(rec,6)}"
                else:
                    target = f"seg{tseg}:{u16b(rec,6):04x}"
            elif t == 1:  # IMPORTORDINAL
                target = apiname(modname(u16b(rec, 4)), u16b(rec, 6))
            elif t == 2:  # IMPORTNAME
                target = f"{modname(u16b(rec,4))}.{impname(u16b(rec,6))}"
            else:
                target = f"OSFIXUP{u16b(rec,4)}"
            # walk the fixup chain (non-additive => location holds next site)
            additive = rel_type & 4
            site = src
            seen = set()
            while site != 0xFFFF and 0 <= site < len(self.data) - 1 and site not in seen:
                seen.add(site)
                self.relocs[site] = target
                self.reloc_kind[site] = addr_type
                if additive:
                    break
                nxt = u16b(self.data, site)
                site = nxt


SEGS = [Seg(i) for i in range(CSEG)]


def entries():
    """Parse the entry table -> list of (ordinal, seg, offset)."""
    out = []
    o = ENTTAB
    ordn = 1
    end = ENTTAB + CBENT
    while o < end:
        cnt = D[o]; seg_ind = D[o + 1]; o += 2
        if cnt == 0:
            continue
        if seg_ind == 0:      # unused bundle
            ordn += cnt
            continue
        if seg_ind == 0xFF:   # movable
            for _ in range(cnt):
                # flags(1) int3f(2) seg(1) off(2)
                segn = D[o + 3]; off = u16(o + 4)
                out.append((ordn, segn, off, "movable"))
                o += 6; ordn += 1
        else:                 # fixed: seg_ind is the segment number
            for _ in range(cnt):
                off = u16(o + 1); o += 3
                out.append((ordn, seg_ind, off, "fixed"))
                ordn += 1
    return out


if __name__ == "__main__":
    print(f"NE@{NE:#x}  segs={CSEG} mods={CMOD} align={1<<ALIGN}  module={pstr(RESTAB)}")
    print("\nSEGMENTS:")
    for s in SEGS:
        print(f"  seg{s.num:<2} {'DATA' if s.is_data else 'CODE'} off={s.file_off:#08x} "
              f"len={s.length:>6} flags={s.flags:#06x} relocs={len(s.relocs)}")
    print(f"\nMODULES: {', '.join(modname(i+1) for i in range(CMOD))}")
    # distinct imported API targets across all segments
    imps = {}
    for s in SEGS:
        for site, tgt in s.relocs.items():
            if "." in tgt and not tgt.startswith("seg"):
                imps[tgt] = imps.get(tgt, 0) + 1
    print(f"\nDISTINCT IMPORTED API CALLS ({len(imps)}):")
    for t, c in sorted(imps.items(), key=lambda kv: (-kv[1], kv[0])):
        print(f"   {c:>4}x  {t}")
    ents = entries()
    print(f"\nENTRY TABLE ({len(ents)} entries):")
    for ordn, segn, off, kind in ents[:30]:
        print(f"   @{ordn:<3} seg{segn}:{off:04x} ({kind})")
