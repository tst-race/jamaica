# Motif Client Control Messages

## Definitions

* Embeddable field - a field that can be used for embedding client control messages. These fields are flagged by the ML stage as `can_embed` in function spec. The current heuristic is large fields containing encrypted data (generally payload fields).

## Handling Embeddable Fields at Arbitrary Locations

The current scheme only supports embedding client control messages as a footer on the packet and will only work in there is an embeddable field as the last field for a packet. However, some protocols may have embeddable fields at arbitrary locations, or multiple non-contiguous embeddable fields. The scheme should be updated to support these types of protocols.

### Server-side Initial Packet Recognition

To handle this, the server should know how to identify the initial packets for the set of protocols that it supports. With this information the server will know where to look in the packets for the client control message(s). The server can then decrypt and decode this initial message which will contain information on the location of embeddable fields in the packets to follow.

The rules for identifying the initial packet should be configurable (e.g. with a file) such that updates and modifications to the protocol can be supported. Initially this configuration will be manually created, but ideally this should be generated in the ML stage for easy support of new protocols.

### Alternative Approaches

#### Heuristic Based Search

Instead of relying on preshared knowledge, the server could alternatively search for the location of embeddable fields. The ML stage can generate a range of possible locations for embeddable fields within a give packet type or protocol. The server can then perform a trial-and-error search, attempting to decrypt and decode fields at the various locations until one is successful.

Note that this assumes no false positives in decrypting and decoding, which may not be true depending on the crypto and encoding used. Would need to verify that Protobuf would support this, as well as whatever encryption is used.
