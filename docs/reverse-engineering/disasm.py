# SPDX-License-Identifier: GPL-3.0-or-later
"""Recursive-descent disassembler for the original SCRANTIC.EXE (NE/Win16), capstone
16-bit, with NE relocations applied so every far call/data ref is labeled (API name or
internal seg:off). Seeds: entry-table exports + every INTERNALREF target. Emits per-segment
annotated .asm, a function list with each function's API/call signature, and a coverage
summary.

Depends on ``ne.py`` (same directory) and ``capstone`` (``pip install capstone``). Inputs
come from the environment (see ``ne.py``); the output dir is::

    DISASM_OUT   where seg*.asm + funcs.txt are written (default: ./out)

The original SCRANTIC.EXE is copyright Sierra/Dynamix and is NOT in this repo; supply your
own copy. See ``README.md``.
"""
import os, re
from collections import defaultdict
import ne
from capstone import Cs, CS_ARCH_X86, CS_MODE_16, x86

md = Cs(CS_ARCH_X86, CS_MODE_16)
md.detail = True

OUT = os.environ.get("DISASM_OUT", "out")
os.makedirs(OUT, exist_ok=True)

# Seeds per segment: exported entries + all INTERNALREF far targets.
seeds = defaultdict(set)
for s in ne.SEGS:
    for off, tgt in s.relocs.items():
        m = re.match(r"seg(\d+):([0-9a-fA-F]+)$", tgt)
        if m:
            seeds[int(m.group(1))].add(int(m.group(2), 16))
for ordn, segn, off, kind in ne.entries():
    seeds[segn].add(off)

ENTRY_NAMES = {  # ordinal -> export name (from the resident-names table order)
    1: "ScreenSaverProc", 2: "ScreenSaverConfigureDialog", 3: "RegisterDialogClasses?",
    4: "DialogProc?", 5: "DialogProc2?"}
entry_label = {}
for ordn, segn, off, kind in ne.entries():
    entry_label[(segn, off)] = ENTRY_NAMES.get(ordn, f"export@{ordn}")


def branch_target(ins):
    if ins.operands and ins.operands[0].type == x86.X86_OP_IMM:
        return ins.operands[0].imm
    return None


total_ins = 0
covered = 0
func_calls = {}   # (seg,off) -> set(targets)
seg_asm = {}

for s in ne.SEGS:
    if s.is_data:
        continue
    code = s.data
    seen = set()
    insns = {}
    work = list(seeds.get(s.num, ()))
    while work:
        a = work.pop()
        if a < 0 or a >= len(code):
            continue
        off = a
        while 0 <= off < len(code) and off not in seen:
            ins = next(md.disasm(bytes(code[off:off + 16]), off), None)
            if ins is None:
                break
            seen.add(off)
            ann = None
            for r in range(off + 1, off + ins.size):
                if r in s.relocs:
                    ann = s.relocs[r]
                    break
            insns[off] = (ins.size, ins.mnemonic, ins.op_str, ann)
            mn = ins.mnemonic
            tgt = branch_target(ins)
            if mn in ("ret", "retf", "iret", "iretd"):
                break
            if mn == "jmp":
                if tgt is not None:
                    work.append(tgt)
                break
            if mn[0] == "j" and tgt is not None:   # conditional jcc / loop
                work.append(tgt)
            if mn in ("call",) and tgt is not None and ann is None:
                work.append(tgt)   # near call inside this segment
            off += ins.size
    total_ins += len(insns)
    covered += sum(v[0] for v in insns.values())
    # group into functions by seed boundaries
    fstarts = sorted(x for x in seeds.get(s.num, ()) if x < len(code))
    for fs in fstarts:
        func_calls.setdefault((s.num, fs), set())
    # write annotated asm + collect per-function calls
    lines = []
    cur_func = None
    fset = set(fstarts)
    for off in sorted(insns):
        if off in fset:
            cur_func = (s.num, off)
            lbl = entry_label.get((s.num, off), "")
            lines.append(f"\n; ---- func seg{s.num}:{off:04x} {lbl} ----")
        sz, mn, ops, ann = insns[off]
        raw = code[off:off + sz].hex()
        c = f"   ; {ann}" if ann else ""
        lines.append(f"  seg{s.num}:{off:04x}  {raw:<16} {mn} {ops}{c}")
        if ann and cur_func is not None:
            func_calls[cur_func].add(ann)
    seg_asm[s.num] = "\n".join(lines)
    open(f"{OUT}/seg{s.num}.asm", "w").write(seg_asm[s.num])

# write function signatures (calls each makes)
with open(f"{OUT}/funcs.txt", "w") as f:
    for (seg, off) in sorted(func_calls):
        calls = sorted(func_calls[(seg, off)])
        apis = [c for c in calls if "." in c and not c.startswith("seg")]
        f.write(f"seg{seg}:{off:04x} {entry_label.get((seg,off),'')}\n")
        if apis:
            f.write("      APIs: " + ", ".join(apis) + "\n")

code_total = sum(s.length for s in ne.SEGS if not s.is_data)
print(f"functions seeded: {len(func_calls)}")
print(f"instructions decoded: {total_ins}")
print(f"code coverage: {covered}/{code_total} bytes ({100*covered/code_total:.1f}%)")
print(f"per-segment .asm written to {OUT}/seg*.asm ; signatures in {OUT}/funcs.txt")
