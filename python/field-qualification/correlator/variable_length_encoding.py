# Copyright 2024 Two Six Technologies.


def detect_quic_var_len_enc(value: int, length: int) -> bool:
    """Detect if an integer is variable length encoded using the QUIC algo. Note that
    false positives are VERY possible.
    https://www.rfc-editor.org/rfc/rfc9000.html#integer-encoding

    Args:
        value (int): The value of the integer.
        length (int): The length of the integer in bits.

    Returns:
        bool: True if it is detected as QUIC style encoding, else False.
    """
    assert value & (pow(2, length) - 1) == 0
    if length == 8:
        return value & 0b11000000 == 0
    elif length == 16:
        return value & 0b1100000000000000 == 0b0100000000000000
    elif length == 32:
        return (
            value & 0b11000000000000000000000000000000
            == 0b10000000000000000000000000000000
        )
    elif length == 64:
        return (
            value & 0b1100000000000000000000000000000000000000000000000000000000000000
            == 0b1100000000000000000000000000000000000000000000000000000000000000
        )

    return False


def detect_google_protobuf_varint(value: int):
    # https://protobuf.dev/programming-guides/encoding/#varints
    varint_bit_mask = 0b01111111

    if value <= varint_bit_mask:
        return True

    while value > 0:
        byte = value & 0x7F
        value >>= 7
        if value == 0:
            return True
        elif byte > varint_bit_mask:
            return False

    return False


def detect_cbor_var_int(value: int) -> bool:
    # https://www.rfc-editor.org/rfc/rfc7049#appendix-B
    if value >= 0 and value <= 23:
        return True

    first_byte = value & 0xFF
    if first_byte == 0x18:
        # First byte is followed by uint8_t
        return value > 0xFF and value & 0x0000 == 0
    elif first_byte == 0x19:
        # First byte is followed by uint16_t
        return value > 0xFFFF and value & 0x000000 == 0
    elif first_byte == 0x1A:
        # First byte is followed by uint32_t
        return value > 0xFFFFFF and value & 0x0000000000 == 0
    elif first_byte == 0x1B:
        # First byte is followed by uint64_t
        return value > 0xFFFFFFFFFF and value & 0x000000000000000000 == 0

    return False


def decode_quic_variable_length_integer(the_int):
    if the_int & 0b1100000000000000000000000000000000000000000000000000000000000000:
        return (
            the_int & 0b0011111111111111111111111111111111111111111111111111111111111111
        )
    elif the_int & 0b10000000000000000000000000000000:
        return the_int & 0b01111111111111111111111111111111
    elif the_int & 0b0100000000000000:
        return the_int & 0b0011111111111111
    else:
        return the_int
