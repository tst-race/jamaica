# Copyright 2024 Two Six Technologies.

"""Labels of known fields in various protocols.
"""

from enum import StrEnum
from collections import defaultdict


# NOTE: simplifying the label set for now to make qualification easier.
class Label(StrEnum):
    UNKNOWN = "unknown"
    LENGTH = "length"
    ID = "id"
    ENCRYPTED = "encrypted"
    FLAG = "flag"
    SEQUENCE = "sequence"
    CONSTANT = "constant"
    # TIMESTAMP = "timestamp" # considering a timestamp as a sequence for now


# DEBUG: wrap a label to signal that a field should be ignored for training and
# testing.
ignore_label = lambda label: f"IGNORE_{label}"

label_map = defaultdict(lambda: Label.UNKNOWN) | {
    "quic.header_form": Label.FLAG,
    "quic.fixed_bit": ignore_label(Label.CONSTANT),
    # Spin Bit
    #
    # https://www.rfc-editor.org/rfc/rfc9000.html#spin-bit
    "quic.spin_bit": ignore_label(Label.FLAG),
    "quic.long.packet_type": Label.FLAG,
    "quic.version": Label.CONSTANT,
    "quic.dcil": Label.LENGTH,
    "quic.dcid": Label.ID,
    "quic.scil": Label.LENGTH,
    "quic.scid": Label.ID,
    # Only present in long header initial, 0-RTT, and handshake packets.
    "quic.length": Label.LENGTH,
    "quic.remaining_payload": Label.ENCRYPTED,
    "quic.long.reserved": Label.CONSTANT,
    "quic.packet_number_length": Label.LENGTH,
    "quic.token_length": Label.LENGTH,
    # QUIC Packet Number
    # From RFC:
    # > This number is used in determining the cryptographic nonce for packet
    # protection.
    # https://www.rfc-editor.org/rfc/rfc9000.html#packet-numbers
    "quic.packet_number": Label.ENCRYPTED,
    "quic.payload": Label.ENCRYPTED,
    # Frame Type
    # Looks like a flag to me. Note that it is variable length encoded.
    # https://www.rfc-editor.org/rfc/rfc9000.html#name-frames-and-frame-types
    "quic.frame_type": Label.FLAG,
    # ACK Largest Acknowledged
    # Corresponds to packet number, so calling this a sequence.
    # Variable length encoded.
    # Appears as constant in our datasets: always zero.
    # https://www.rfc-editor.org/rfc/rfc9000.html#name-ack-frames
    # TODO: ignore for now. Not sure how to label this or how often it shows up in
    # practice.
    "quic.ack.largest_acknowledged": ignore_label(Label.SEQUENCE),
    # ACK Delay
    # Measure of delay in microseconds.
    # Might ignore for now, we don't have a label for this.
    # Variable length encoded.
    "quic.ack.ack_delay": ignore_label(""),
    "quic.ack.ack_range_count": ignore_label(""),
    "quic.ack.first_ack_range": ignore_label(""),
    # QUIC Token
    # Looks encrypted to me from spot-checking PCAPs. RFC is a bit vague. Used to
    # validate client address?
    # Also seeing one in a handshake packet, but RFC doesn't even show this field for
    # handshake packets...
    # https://www.rfc-editor.org/rfc/rfc9000.html#name-initial-packet
    "quic.token": Label.ENCRYPTED,
    # QUIC Crypto Frame Offset
    # Variable length encoded.
    # https://www.rfc-editor.org/rfc/rfc9000.html#name-crypto-frames
    # TODO: not really sure. Length?
    "quic.crypto.offset": ignore_label(""),
    "quic.crypto.length": Label.LENGTH,
    ####################################################################################
    # RTP
    # https://www.rfc-editor.org/rfc/rfc3550.html
    ####################################################################################
    "rtp.version": Label.CONSTANT,
    "rtp.padding": Label.FLAG,
    "rtp.ext": ignore_label(Label.FLAG),
    # RTP CSRC count (CC)
    # https://www.rfc-editor.org/rfc/rfc3550.html#section-5.1
    # 4 least sig bits of the first byte.
    "rtp.cc": ignore_label(Label.LENGTH),  # could also be flag, it's only four bits.
    "rtp.marker": Label.FLAG,
    "rtp.p_type": Label.FLAG,
    "rtp.seq": Label.SEQUENCE,
    "rtp.extseq": Label.SEQUENCE,
    "rtp.timestamp": Label.SEQUENCE,  # Label.TIMESTAMP,
    # RTP SSRC
    # Sounds like an ID to me. From the docs:
    # > The SSRC identifier carried in the RTP header and in various fields of RTCP
    # packets is a random 32-bit number that is required to be globally unique within an
    # RTP session. It is crucial that the number be chosen with care in order that
    # participants on the same network or starting at the same time are not likely to
    # choose the same number.
    # https://www.rfc-editor.org/rfc/rfc3550.html#section-8
    "rtp.ssrc": Label.ID,
    "rtp.padding.data": Label.ENCRYPTED,
    "rtp.padding.count": Label.LENGTH,
    "rtp.csrc.item": Label.ID,
    "rtp.payload": Label.ENCRYPTED,
    ##########################################
    # RTP header extensions
    # https://www.rfc-editor.org/rfc/rfc3550.html#section-5.3.1
    # https://www.rfc-editor.org/rfc/rfc5285
    ##########################################
    # RTP header extension profile
    # https://www.rfc-editor.org/rfc/rfc5285#section-4.2
    # Values is either:
    #   * 0xBEDE and 8-bits long (one-byte header)
    #   * 0x100  and 6-bits long (two-byte header)
    # Effectively a flag signaling a one-byte header or two-byte header.
    "rtp.ext.profile": Label.FLAG,
    # From the docs:
    # > a 16-bit length field that counts the number of 32-bit words in the extension,
    # > excluding the four-octet extension header (therefore zero is a valid length).
    "rtp.ext.len": Label.LENGTH,
    # Sets the extension header type. Really more like a flag.
    "rtp.ext.rfc5285.id": Label.FLAG,  # Label.ID,
    # From the docs:
    # > The 4-bit length is the number minus one of data bytes of this header extension
    # > element following the one-byte header.  Therefore, the value zero in this field
    # > indicates that one byte of data follows, and a value of 15 (the maximum)
    # > indicates element data of 16 bytes.
    "rtp.ext.rfc5285.len": Label.LENGTH,
    "rtp.ext.rfc5285.data": Label.ENCRYPTED,
    # From the docs:
    # > The appbits field is 4 bits that are application-dependent and may be defined to
    # be any value or meaning, and are outside the scope of this specification.
    # 4-bits long.
    # Appears to always be zero in our captures.
    "rtp.ext.rfc5285.appbits": Label.CONSTANT,
    ####################################################################################
    # SRTP
    # https://www.rfc-editor.org/rfc/rfc3711.html
    ####################################################################################
    "srtp.enc_payload": Label.ENCRYPTED,
    # Is really more of a checksum than encrypted data. Need to think about this one.
    "srtp.auth_tag": ignore_label(Label.ENCRYPTED),


    # "dtls.app_data": Label.ENCRYPTED,
    # "dtls.handshake.cert_type": Label.FLAG,
    # "dtls.handshake.cert_types_count": Label.LENGTH,
    # "dtls.handshake.certificate": Label.ENCRYPTED, # TODO: is this right?
    # "dtls.handshake.certificate_length": Label.LENGTH,
    # "dtls.handshake.certificates_length": Label.LENGTH,
    # "dtls.handshake.cipher_suites_length": Label.LENGTH,
    # "dtls.handshake.ciphersuite": Label.FLAG,
    # "dtls.handshake.client_cert_vrfy.sig": Label.ENCRYPTED, # TODO: is this a checksum?!
    # "dtls.handshake.client_cert_vrfy.sig_len": Label.LENGTH,
    # "dtls.handshake.client_point": Label.ENCRYPTED, # TODO: not sure, not showing up in WireShark
    # "dtls.handshake.client_point_len": Label.LENGTH,
    # "dtls.handshake.comp_method": Label.FLAG, # TODO: not sure
    # "dtls.handshake.comp_methods_length": Label.LENGTH,
    # "dtls.handshake.cookie_length": Label.LENGTH,
    # "dtls.handshake.dnames_len": Label.LENGTH,
    # "dtls.handshake.extension.len": Label.LENGTH,
    # "dtls.handshake.extension.type": Label.FLAG,
    # "dtls.handshake.extensions_ec_point_format": Label.FLAG,
    # "dtls.handshake.extensions_ec_point_formats_length": Label.LENGTH,
    # "dtls.handshake.extensions_length": Label.LENGTH,
    # "dtls.handshake.extensions_reneg_info_len": Label.LENGTH,
    # "dtls.handshake.extensions_supported_group": Label.FLAG,
    # "dtls.handshake.extensions_supported_groups_length": Label.LENGTH,
    # "dtls.handshake.fragment_length": Label.LENGTH,
    # "dtls.handshake.fragment_offset": Label.LENGTH, # TODO: this is the closest, but we don't really have a label for this.
    # "dtls.handshake.length": Label.LENGTH,
    # "dtls.handshake.message_seq": Label.SEQUENCE,
    # "dtls.handshake.random": Label.ENCRYPTED, # TODO: closest we have, effectively the same atm
    # "dtls.handshake.random_bytes": Label.ENCRYPTED,
    # "dtls.handshake.random_time": Label.ENCRYPTED,
    # "dtls.handshake.server_curve_type": Label.FLAG,
    # "dtls.handshake.server_named_curve": Label.FLAG, # TODO: not sure
    # "dtls.handshake.server_point": Label.ENCRYPTED,
    # "dtls.handshake.server_point_len": Label.LENGTH,
    # "dtls.handshake.session_id_length": Label.LENGTH,
    # "dtls.handshake.session_ticket": Label.ENCRYPTED, # TODO: maybe?
    # "dtls.handshake.session_ticket_length": Label.LENGTH,
    # "dtls.handshake.session_ticket_lifetime_hint": Label.CONSTANT, # TODO: not constant really, although maybe constant per implementation
    # "dtls.handshake.sig": Label.ENCRYPTED, # TODO: checksum?!
    # "dtls.handshake.sig_hash_alg": Label.FLAG,
    # "dtls.handshake.sig_hash_alg_len": Label.LENGTH,
    # "dtls.handshake.sig_hash_hash": Label.FLAG,
    # "dtls.handshake.sig_hash_sig": Label.FLAG,
    # "dtls.handshake.sig_len": Label.LENGTH,
    # "dtls.handshake.type": Label.FLAG,
    # "dtls.handshake.version": Label.CONSTANT, # TODO: make sure it's constant across datasets
    # "dtls.record.content_type": Label.FLAG,
    # # From the docs:
    # #   A counter value that is incremented on every cipher state change.
    # # So, not really a flag, but close enough in our datasets.
    # "dtls.record.epoch": Label.FLAG,
    # "dtls.record.length": Label.LENGTH,
    # "dtls.record.sequence_number": Label.SEQUENCE,
    # "dtls.record.version": Label.CONSTANT, # TODO: make sure it's constant across datasets,
    # "dtls.unknown": Label.UNKNOWN,
    # "dtls.use_srtp.mki_length": Label.LENGTH,
    # "dtls.use_srtp.protection_profile": Label.FLAG,
    # "dtls.use_srtp.protection_profiles_length": Label.LENGTH,
}
