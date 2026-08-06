ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def b58encode(data: bytes) -> str:
    if not data:
        return ""

    value = int.from_bytes(data, "big")
    encoded = ""
    while value:
        value, remainder = divmod(value, 58)
        encoded = ALPHABET[remainder] + encoded

    leading_zeroes = len(data) - len(data.lstrip(b"\0"))
    return "1" * leading_zeroes + encoded


def b58decode(text: str) -> bytes:
    if not text:
        return b""

    value = 0
    for char in text:
        index = ALPHABET.find(char)
        if index < 0:
            raise ValueError(f"Invalid base58 character {char!r}")
        value = value * 58 + index

    decoded = value.to_bytes((value.bit_length() + 7) // 8, "big") if value else b""
    leading_ones = len(text) - len(text.lstrip("1"))
    return b"\0" * leading_ones + decoded
