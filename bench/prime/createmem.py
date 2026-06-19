with open("prime.bin", "rb") as f:
    data = f.read()

with open("prime.hex", "w") as out:
    # Pad to a multiple of 4 bytes
    while len(data) % 4:
        data += b'\x00'
    for i in range(0, len(data), 4):
        word = int.from_bytes(data[i:i+4], "little")  # RISC-V is little-endian
        out.write(f"{word:08x}\n")
