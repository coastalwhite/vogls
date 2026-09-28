"""Turn trva's section dump into the $readmemh image that tb.v loads.

`just build` runs trva over prime.S and leaves its section dump in
prime.trva; this script flattens the loadable sections into prime.hex,
one little-endian 32-bit word per line.
"""

import re
import sys

DUMP = "prime.trva"
HEX = "prime.hex"

SECTION = re.compile(r"^-- \.(\w+) \((0x[0-9A-Fa-f]+)-(0x[0-9A-Fa-f]+)\) --$")

# .bss is zero-initialised, so trva emits only its bounds and no bytes.
LOADED = ("text", "data", "rodata")

# tb.v backs addresses below this with `memory`, and everything at or above
# it with the separate `primes` array -- which is also where .bss lives.
MEMORY_BYTES = 1024


def parse_sections(dump):
    """Return a (name, start address, bytes) triple per loadable section."""
    sections = []
    lines = dump.splitlines()
    i = 0

    while i < len(lines):
        line = lines[i].strip()
        i += 1

        match = SECTION.match(line)
        if match is None:
            sys.exit(f"createmem: unexpected line in {DUMP}: {line!r}")

        name, start = match.group(1), int(match.group(2), 16)
        if name not in LOADED:
            continue

        if i >= len(lines):
            sys.exit(f"createmem: {DUMP} ends before the .{name} contents")
        sections.append((name, start, bytes.fromhex(lines[i].strip())))
        i += 1

    return sections


def build_image(sections):
    """Lay the sections out at their absolute addresses, zero-filling gaps."""
    image = bytearray()

    for name, start, data in sections:
        if not data:
            continue
        end = start + len(data)
        if end > MEMORY_BYTES:
            sys.exit(
                f"createmem: .{name} ends at {end:#x}, past the {MEMORY_BYTES:#x} "
                f"bytes tb.v backs with `memory`"
            )
        if start < len(image):
            sys.exit(f"createmem: .{name} at {start:#x} overlaps an earlier section")
        image.extend(bytes(start - len(image)))
        image.extend(data)

    return image


with open(DUMP) as f:
    image = build_image(parse_sections(f.read()))

# Pad to a whole number of words
image.extend(bytes(-len(image) % 4))

with open(HEX, "w") as out:
    for i in range(0, len(image), 4):
        word = int.from_bytes(image[i : i + 4], "little")  # RISC-V is little-endian
        out.write(f"{word:08x}\n")
