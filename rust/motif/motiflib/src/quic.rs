// Copyright 2024 Two Six Technologies.

use hex;
use num;

use rand;

use crate::action::{Action, SendAction};

// TODO: temporary hardcodings from sample PCAP
// const SERVER_CONNECTION_ID: &str = "f25b987a5d73ccdf";

const INITIAL_PACKET_SIZE: usize = 1200;

const QUIC_VERSION: u32 = 1;

// pub struct QuicInput {
//     actions: VecDeque<Action>,
// }

// impl QuicInput {
//     pub fn new() -> Self {
//         Self {
//             actions: vec![
//                 create_quic_initial_packet().into(),
//                 Action::Receive(ReceiveAction::new(5)),
//                 Action::Receive(ReceiveAction::new(5)),
//                 create_quic_handshake_packet(SERVER_CONNECTION_ID, CLIENT_CONNECTION_ID, 1).into(),
//                 create_quic_protected_payload_packet(1326, SERVER_CONNECTION_ID).into(),
//                 create_quic_protected_payload_packet(157, SERVER_CONNECTION_ID).into(),
//                 Action::Receive(ReceiveAction::new(5)),
//                 Action::Receive(ReceiveAction::new(5)),
//                 Action::Receive(ReceiveAction::new(5)),
//                 Action::Quit,
//             ]
//             .into(),
//         }
//     }
// }

// impl MotifInput for QuicInput {
//     fn get_next_action(&mut self) -> Action {
//         self.actions.pop_front().expect("ahh dammit")
//     }
// }

impl Into<Action> for Vec<u8> {
    fn into(self) -> Action {
        // Action::Send(SendAction::new(self))
        Action::Send(SendAction {
            // payload: self,
            ..Default::default()
        })
    }
}

// https://www.rfc-editor.org/rfc/rfc9000.html#name-long-header-packets
pub struct QuicLongHeader {
    fixed_bit: u8, // one bit
    #[allow(dead_code)]
    packet_number_length: u8, // two bits.
    /// Debug hack to allow explicitly setting the first byte to work around header protection.
    first_byte: Option<u8>,
    version: u32,
    destination_connection_id: String,
    source_connection_id: String,
}

impl Default for QuicLongHeader {
    fn default() -> Self {
        Self {
            fixed_bit: 1,
            packet_number_length: 0,
            first_byte: None,
            version: QUIC_VERSION,
            destination_connection_id: "".to_string(),
            source_connection_id: "".to_string(),
        }
    }
}

impl QuicLongHeader {
    fn to_vec(&self) -> Vec<u8> {
        let mut header = vec![];

        // TODO: remove `first_byte` hack and implement header protection.
        let first_byte = if let Some(first_byte) = self.first_byte {
            first_byte
        } else {
            let header_form: u8 = 1 << 7;
            let fixed_bit: u8 = (self.fixed_bit as u8) << 6;
            header_form | fixed_bit
        };

        header.push(first_byte);

        header.extend(self.version.to_be_bytes());

        // Destination connection length and the ID itself.
        let destination_connection_id =
            hex::decode(&self.destination_connection_id).expect("Decoding failed");
        debug_assert!(destination_connection_id.len() <= 20);
        let destination_connection_id_length = destination_connection_id.len() as u8;
        header.extend(destination_connection_id_length.to_be_bytes());
        header.extend(destination_connection_id);

        // Source connection ID length and the ID itself.
        let source_connection_id =
            hex::decode(&self.source_connection_id).expect("Decoding failed");
        debug_assert!(source_connection_id.len() <= 20);
        let source_connection_id_length = source_connection_id.len() as u8;
        header.extend(source_connection_id_length.to_be_bytes());
        header.extend(source_connection_id);

        header
    }
}

/// https://www.rfc-editor.org/rfc/rfc9000.html#name-short-header-packets
struct QuicShortHeader {
    /// Debug hack to allow explicitly setting the first byte to work around header protection.
    first_byte: Option<u8>,
    destination_connection_id: String,
}

impl Default for QuicShortHeader {
    fn default() -> Self {
        Self {
            first_byte: Some(0),
            destination_connection_id: "".to_string(),
        }
    }
}

impl QuicShortHeader {
    fn to_vec(&self) -> Vec<u8> {
        let mut header = vec![];

        // TODO: hardcoding to match sample PCAP. Need to implement header protection.
        let first_byte = if let Some(first_byte) = self.first_byte {
            first_byte
        } else {
            todo!("ahhhhh!");
        };
        header.push(first_byte);

        let destination_connection_id =
            hex::decode(&self.destination_connection_id).expect("Decoding failed");
        header.extend(destination_connection_id);

        header
    }
}

pub struct QuicInitialPacketPartial {
    header: QuicLongHeader,
    /// Hex string.
    token: String,
    // Between 8 and 32 bits.
    #[allow(dead_code)]
    packet_number: u32,
}

/// https://www.rfc-editor.org/rfc/rfc9000.html#name-initial-packet
///
/// Include the following fields
/// Initial Packet Partial {
///   Header Form (1) = 1,
///   Fixed Bit (1) = 1,
///   Long Packet Type (2) = 0,
///   Reserved Bits (2),
///   Packet Number Length (2),
///   Version (32) = 1,
///   Destination Connection ID Length (8),
///   Destination Connection ID (0..160),
///   Source Connection ID Length (8),
///   Source Connection ID (0..160),
///   Token Length (i),
///   Token (..),
/// }
impl Default for QuicInitialPacketPartial {
    fn default() -> Self {
        Self {
            header: QuicLongHeader::default(),
            token: "".to_string(),
            packet_number: 0,
        }
    }

    // TODO: add a from method to create an instance from a Vec<u8> ???
}

pub fn is_quic(packet: &Vec<u8>) -> bool {
    true
}

pub fn get_destination_connection_id_from_initial_packet(initial_packet: &Vec<u8>) -> String {
    const DESTINATION_CONNECTION_ID_LENGTH_INDEX: usize = 5;
    let destination_connection_id_length =
        initial_packet[DESTINATION_CONNECTION_ID_LENGTH_INDEX] as usize;
    const DESTINATION_CONNECTION_ID_INDEX: usize = 6;
    let end_index = DESTINATION_CONNECTION_ID_INDEX + destination_connection_id_length;
    let connection_id = &initial_packet[DESTINATION_CONNECTION_ID_INDEX..end_index];
    hex::encode(connection_id)
}

pub fn get_destination_connection_id_from_short_header(initial_packet: &Vec<u8>) -> String {
    const DESTINATION_CONNECTION_ID_INDEX: usize = 1;
    // TODO: assuming the length is always 8 bytes. According to RFC it can vary, although idk how
    //       you tell without a length field.
    const DESTINATION_CONNECTION_ID_LENGTH: usize = 8;
    let end_index = DESTINATION_CONNECTION_ID_INDEX + DESTINATION_CONNECTION_ID_LENGTH;
    let connection_id = &initial_packet[DESTINATION_CONNECTION_ID_INDEX..end_index];
    hex::encode(connection_id)
}

pub fn get_source_connection_id_from_initial_packet(initial_packet: &Vec<u8>) -> String {
    const DESTINATION_CONNECTION_ID_LENGTH_INDEX: usize = 5;
    let destination_connection_id_length =
        initial_packet[DESTINATION_CONNECTION_ID_LENGTH_INDEX] as usize;
    let source_connection_id_length_index: usize =
        DESTINATION_CONNECTION_ID_LENGTH_INDEX + 1 + destination_connection_id_length;
    let source_connection_id_length = initial_packet[source_connection_id_length_index] as usize;
    let source_connection_id_index: usize = source_connection_id_length_index + 1;
    let end_index = source_connection_id_index + source_connection_id_length;
    let connection_id = &initial_packet[source_connection_id_index..end_index];
    hex::encode(connection_id)
}

pub fn get_destination_connection_id_from_packet(packet: &Vec<u8>) -> String {
    let is_long_header: bool = packet[0] & 0b1000_0000 != 0;
    if is_long_header {
        return get_destination_connection_id_from_initial_packet(packet);
    } else {
        return get_destination_connection_id_from_short_header(packet);
    }
}

impl QuicInitialPacketPartial {
    fn to_vec(&self) -> Vec<u8> {
        let mut header = self.header.to_vec();

        let token_length = self.token.len() / 2;
        header.extend(token_length.to_variable_length_integer());

        if token_length > 0 {
            header.extend(hex::decode(&self.token).expect("decode failed"));
        }

        header
    }
}

pub fn get_random_hex_string(size: usize) -> String {
    let random_hex: Vec<u8> = (0..size).map(|_| rand::random()).collect();
    hex::encode(random_hex)
}

fn append_hex_string_to_packet(packet: &mut Vec<u8>, hex_string: &str) {
    packet.extend(hex::decode(hex_string).expect("decoding failed"));
}

/// https://www.rfc-editor.org/rfc/rfc9000.html#name-initial-packet
pub fn create_quic_initial_packet(source_connection_id: &str) -> Vec<u8> {
    // TODO: not sure how the initial destination connection ID is generated by the client. Seems
    //       to be totally ignored by ther server for both Quinn and Instagram.
    let destination_connection_id = get_random_hex_string(20);
    let source_connection_id = source_connection_id.to_string();
    let mut packet = QuicInitialPacketPartial {
        header: QuicLongHeader {
            first_byte: Some(193), // TODO: hardcoded from sample PCAP
            destination_connection_id,
            source_connection_id,
            ..Default::default()
        },
        ..Default::default()
    }
    .to_vec();

    // Length
    // https://www.rfc-editor.org/rfc/rfc9000.html#name-variable-length-integer-enc
    // Subtract 2 for the size of the length field itself.
    // TODO: just assuming that length will always be two bytes. _Should_ hold true...
    let length = (INITIAL_PACKET_SIZE - packet.len() - 2).to_variable_length_integer(); // TODO: what should this be?
    debug_assert_eq!(length.len(), 2);
    packet.extend(length);

    // Packet number
    // packet.push(packet_number);
    // TODO: for now hardcode based on sample PCAP until we implement header protection.
    packet.push(130);

    // Create payload.
    // TODO: for now hardcode based on sample PCAP
    // packet.extend(hex::decode("0360c55b85ec880a163d76ee6d60c9e80a20c8c263d3132ba88f83177a7510e8496369190a0496f6f67006788014b2d61ee8e2411a80569f728119aef68f4c3e354ff1bf272b9f03f1d00efaaa6816327c06fa16784d965868312b33735a45d2725e4f3f15d033e54a276320937f2ffe29f0d120f0c038b6fcfad944ae4011a94421ba768e087e032fe20ba0e9eefc7fdd485cd0c44ac884af0410b29fa1e17de8b25699cb6e7d825e20356c871057ad36ba6f0b95d07a90c11e8cc025e093c22fea7ecbbe055a6341e2d1c398d254553268220759adbb90a6de7b3865372035ec804eda33aa9b53ea373770f437782ca66a7ae2aded0acd9b6bf5f8487c3930c967eb473f4043e4f15bf365288bb468edef0a7e9be07fdb519de474f878ca7d5dd02e48dd2d36a4086934aef20240eac3e81e3fd5b1041c052a84318ea68fa051003aa04d28f623849b1a4ce66824981c86102114219aaf8490a34bf0c0f2f1d43a62c76fb02f1a005e1495634a38d77754cfa8b55f397d4d4dc83147d2d168aa7b4e5b52820ba9c154501e6fc06cbc8d0dda1c3c03a9da6e499451bf955c5ca7e6acc058744cb576b801afc7403e945c8af435eedb311b99e6cc4b0606a5bf1cd9933f04a3924d6a3fb7d1fff4b83efe3b1d9d3c41356c13fb626c90ca407cd44e5c8e3f81c496ec9ede504781df1071440b90e17742d31153153e30991824fe33d6a6377b07bd83bf187da688577a73af3b87608418fac0c587efa5ed496010d83455f47f1ff1d66e2ac1f94ea8c0a361b25985c8c13a6e2891a8a5a16e30e6756783aafbea933d6beab2621b44772759b53bb177caf532082a8809e57cf5d18eaca64c8fb5d0492f724db3ab8b294db23b57f10f653abd1e351b2793cc3a9c6ede01a49763a8b42f92de5607319a8893d983b77d21c464a5800eeb0a7aa5669e73117a8cef99ccb73cc79c69757af6621bee30f76d8cb33545892aaf8e03923bda0d9ae17e914de82bd0dc5f113491840daf4f2f74d90a82bd395c9709967a1c480ac6ab72c6b3a08487c113b42f085bf6ab7571e1383842c94b692ca66fbc537951fd69294a4548e3439153d698efa7e3f7a87b74eb931d0456e5e0ac8bc2c5c26c57c6a62b0ccca92a9ac36a6420d0cce62fc86e198752e5e60519bef186f89b92cab1d01079e742da5ef9ca9aeb93a3fd0ef4e64a3de62f0a1a1054e1ce42682ca0ed80abc14e3d09e40f4877cff1a50fdec54e011965c5c52c929c443059bd027eb9485c4ca013c41db29be9d06bf010ae50e46676423befaaf4d116bf9fdebe0bbba481a5b185bec6ad109392cef5abfea96325c86c8541f0801e4e317fdd93892ff65c15f697eecc368dbe360c817ff37c8b903dd217d14bbea26877111a5be65cdda871f2f755d2eddf15d99a865701bc7f2b1cce0adcbcf68e3c51fa06fc3db3440f77381b8fa3d4890137cd01f2ba563f5d91c2233d752092fd913fb69ffa47623aa411e5757d26d432a559fb1972242715c450f2b30ca028deea7dfc205445cf51141aaf7db85b297acac42322f680d58961ea4c9d769dc7a6c20c33f739f18a01b80c7feb179afb45db6f5077f4cfca52b6a4fb288f461e177f7a075cc1f1c9ba11").expect("decoding failed"));
    append_hex_string_to_packet(&mut packet, "0caab786f002123719431e7ed743dc557a692e61fc6f4b5ab9979d2195633cd9e8bb163cf32ed487a85f301a834adcd6543f2fa0852a413fa00b8ace73ce60284a1cb0e2a092129e6536cf0e95255034a7b35584bb304656967f290e592cc4c4c558e5e63eb7fb8cfbb910f7dd2295c9e0eb283b9aa586b3ddc8a8e590e304d645180a078c05aa9175bf2e8300fd4bb012051fea3ac92d2c625c8eff70b993a11452b6a409ffb59dae130e5a4a2826b966dfa5d85c91da55c904d44601df3de067ab5416c359934696addbefb3fb82b68c22405999c362199cd71dc25613c5ab60c0ecbe931b9c3d3d2f4d921e9f94748f37b5c7759636d568d198c035a6bd2910a1e7a6671f58034b602dc75bae6ece6ebc7de9e2630e08d5bfba245cb9f98e437aaa724031009be886e065e31ec8013385c798f578c650261c227afe61c15946e760ebd63f03c9a954e7b58e370eb7233e37f41f90bd8cdf6a6397158c7b593c3eae95f6f7f03cbaadc46af6bb5dcabe6c201a4cf14d3213bec55c384ac97c59092a41ef853a87106d1577f1b379efab0cf61ba05193e43d5f2d03deb319eb8ba7faaf4781eeb98ce29b7b6cc3ceb19bec0506965bdcd4237fc963b0e039fee11dcc7bee572c4672ba138f184de97b4071572bc563018a468641d7dbedcfbff74dc6ec2addec38e83ce187e325a0cba58af0efa41d831e330be149132b327650ada6f2766f7ce15921cd24ce10b8278057cd0f61747942c540bba2c7aadfa8f79b1e5bd01c798a924399607ef6599ea5484081514e0952a7519aae8656941a3fe340eb562b4fb45f05d423daa84212949422bb5eded011c71dccbf4c8f2f75036914f8916f91082504a5593685c61ee8ee608db8f8addcf75173a4317a5dd2a06077bcbdf63884e7387d7cfc9aa432b1f037d6ac0adbfb6aeee23383f5096f653b919fbf8719ae0d7a233e1c3da853b274f8a219937835c8380602568dbd81d11ac4bc296c9bf12b0094bdee003879642e5841ff021e611481c2262174f5dbdc64a5bdbb46f22f6c821b1b139e1f41627ad0ea023c4c373797db7e86bfbf9f0e5d467b24648a22423a50f23af85eb2c706072684d64efd26167480b24a3e5070752d0e0621204fdd0edcb429d9173c667e88c6edacb08033c921acefa65026d32380c14f21a599305e81cb3e4ce7bf76ca925484bb75912d27b13135d890d4a5d73504b476478222ee76015b0f4328199e4be003d90a612ddae58c8c38f50d4d11717fe49e2578d82afc133541d47c7526a8dfc5c56e505f6c6ffcfe27c7f81ba7f3a0f270d5c7515646a3fac76bd2a056a94a8b742b1fb95966523748fbc37898dae31caec0e2e76330b0c784e8e0d2c94b117af4ec9db1e983e4b2d44c8ee24fd4e7c742ec36a8ecc2c88efa04a06a0af027e1de9e2ef658b7ce8a262b9fce0360c4a46c25479ae7ca3a0c090a8e768ecb08d890011cc43a5366441566c2f23541cc5fce1b0e3a198ac555f5646ce75f35ae65a6b0da6cdc9c8b16ba6067df3ddfc4c72b0a7659eb300a3ced2d9cf95d20cbf8af9f2a472afb6ba952ecae9cf22769542e336e40b444412ab03be76ad1611052e376bdfe22aa590727ba473094225fbdc8e5fd25");

    debug_assert!(packet.len() <= INITIAL_PACKET_SIZE);

    // // TESTING: gutting out the end of the payload
    // let length = packet.len();
    // packet[length - 16 ..].copy_from_slice(vec![64; 16].as_slice());

    // Create crypto frame
    // packet.extend(create_crypto_frame());

    // pad with randomness.
    // TODO: this causes a bug in wireshark. Fix this.
    // [Dissector bug, protocol QUIC: /Users/buildslave/builds/UfJL1hoT/0/wireshark/wireshark/epan/dissectors/packet-quic.c:4252: failed assertion "quic_packet"]
    let padding_size = INITIAL_PACKET_SIZE - packet.len();
    // let payload: Vec<u8> = (0..padding_size).map(|_| rand::random()).collect();
    // packet.extend(payload);

    // pad with zeros.
    packet.extend(vec![0; padding_size]);

    packet
}

/// https://www.rfc-editor.org/rfc/rfc9000.html#name-variable-length-integer-enc
pub trait VariableLengthEncode {
    fn to_variable_length_integer(self) -> Vec<u8>;
}

impl<T: num::ToPrimitive> VariableLengthEncode for T {
    fn to_variable_length_integer(self) -> Vec<u8> {
        debug_assert!(
            self.to_u128().expect("") <= 4611686018427387903u128,
            "exceeded max value from RFC"
        );
        let value = self
            .to_u64()
            .expect("failed to encode variable length integer");
        match value {
            value if value > 1073741823 => (value as u64 | (0b11 << 62)).to_be_bytes().to_vec(),
            value if value > 16383 => (value as u32 | (0b10 << 30)).to_be_bytes().to_vec(),
            value if value > 63 => (value as u16 | (0b01 << 14)).to_be_bytes().to_vec(),
            _ => vec![value as u8],
        }
    }
}

// TODO: not used yet, still hardcoding things before implementing real handshake+encryption.
/// https://www.rfc-editor.org/rfc/rfc9000.html#name-crypto-frames
#[allow(dead_code)]
fn create_crypto_frame() -> Vec<u8> {
    // let mut frame = vec![];

    // let frame_type: u8 = 0x06;
    // frame.push(frame_type);

    // let offset: u8 = 0;
    // frame.push(offset);

    // // TODO: hardcoded value taken from PCAP. what should the value actually be?
    // let length = 273;
    // frame.extend(create_variable_length_integer(length));

    // let crypto_data: Vec<u8> = (0..length).map(|_| rand::random()).collect();
    // frame.extend(crypto_data);

    // frame

    unimplemented!();
}

// TODO: not used yet, still hardcoding things before implementing real handshake+encryption.
#[allow(dead_code)]
fn create_ack_frame() -> Vec<u8> {
    let mut frame: Vec<u8> = vec![];

    // ACK frame.
    let frame_type = 3u8;
    frame.push(frame_type);

    let largest_acknowledged = 0u8;
    frame.push(largest_acknowledged);

    let ack_delay = 127u8; // TODO: what should this be?
    frame.push(ack_delay);

    let ack_range_count = 0u8;
    frame.push(ack_range_count);

    let first_ack_range = 0u8;
    frame.push(first_ack_range);

    let ect0_count = 1u8;
    frame.push(ect0_count);

    let ect1_count = 0u8;
    frame.push(ect1_count);

    let ecn_ce_count = 0u8;
    frame.push(ecn_ce_count);

    frame
}

/// https://www.rfc-editor.org/rfc/rfc9000.html#name-handshake-packet
/// The handshake packet is actually two QUIC packets: an initial packet followed by a handshake
/// packet.
pub fn create_quic_handshake_packet(
    destination_connection_id: &str,
    source_connection_id: &str,
    _packet_number: u8,
) -> Vec<u8> {
    let mut packet = QuicInitialPacketPartial {
        header: QuicLongHeader {
            // TODO: this value is hardcoded from the sample PCAP. Note that quinn is incorrectly
            // setting the packet type for handshake packets, so I've altered this from 8d to ad.
            // Value currently hardcoded since we have yet to implement header protection.
            first_byte: Some(hex::decode("ad").expect("noooo")[0]),
            destination_connection_id: destination_connection_id.to_string(),
            source_connection_id: source_connection_id.to_string(),
            ..Default::default()
        },
        packet_number: 130,
        ..Default::default()
    }
    .to_vec();

    // Length.
    // TODO: hardcoded length from sample PCAP.
    packet.extend(120.to_variable_length_integer());

    // Packet number.
    // TODO: hardcoded from sample PCAP with packet protection already applied.
    packet.extend(hex::decode("5c").expect("decode failed"));

    // TODO: appending hardcoded payload for ACK and CRYPTO frame from sample PCAP
    // packet.extend(hex::decode("139f8035c7a5942515dbdd660c111b6c53dab4ac7774dbd55ca7050230558e56daa70674fc0ebd61754da6257edbe7377478d0e97f6e3882f7376e651e2adf7fbac69dbdf7aa58d47ce47425019d00332f6a1ae12109a2bb14eac2c13a8c9f80a2bbed0a9f876a19902ce366105e9cf6aa5eea38f995a0").expect("decode failed"));

    let payload: Vec<u8> = (0..238).map(|_| rand::random()).collect();
    packet.extend(payload);

    // packet.extend(create_ack_frame());

    // packet.extend(create_crypto_frame());

    let header = QuicLongHeader {
        fixed_bit: 1,
        first_byte: Some(237), // TODO: hardcoded from sample PCAP to work around header protection.
        destination_connection_id: destination_connection_id.to_string(),
        source_connection_id: source_connection_id.to_string(),
        ..Default::default()
    };

    packet.extend(header.to_vec());

    // TODO: append actual length
    // Subtract 2 for the size of the length field itself.
    // TODO: just assuming that length will always be two bytes. _Should_ hold true...
    let length = INITIAL_PACKET_SIZE - packet.len() - 2; // TODO: what should this be?
                                                         // debug_assert_eq!(length, 1029); // TODO: hardcoded value from sample PCAP
    let length = length.to_variable_length_integer();
    debug_assert_eq!(length.len(), 2);
    packet.extend(length);

    // Token length

    // Token

    // Length

    // Packet number

    // Payload

    //      Frame Type

    const PAYLOAD_SIZE: usize = 1200;
    // pad with randomness.
    let padding_size = PAYLOAD_SIZE - packet.len();
    let payload: Vec<u8> = (0..padding_size).map(|_| rand::random()).collect();
    packet.extend(payload);

    // TODO: append hardcoded payload from sample packet to workaround header protection
    // packet.extend(hex::decode("e122a6849b1f523a94a31d753c073a547a8649957f3e5a76bdb7be525d7a9397e970c8f956515830168017d7766782385235137ef1732cbbf9748ec568a27354451d31a989769e67bddad764e849d9efe1953986f5638b4eb9b2bc71d0b288e8dcf3d6a06ec7f1896b136cc03423bd9f10a70cc841a67381229171dbb0b809eaef8a26dde14292476026114657317ff706c2c4d021e81ab48c680d43fc791810dfe4d6fb23d44a7e463c30339c319033a1783a1a8c3fca86242b75c8741f5016fc9f3f32ac379c5696e0bf61571fa3e261f18fa7edfaccdd0ee5a1106f858a04981fb8187753b2012c21c7cd1896c92a69cf3b312290554f365ce54504d265727635c4c0816917cb36b11d0e76d16ffc79bb91b8222b964f705ed63818a51cd3576a74ab11b44807719cea37ef1525ab42d164dc76d0b4d8e19918b845f1d3163a9293c74cc31e185a11d501f63f7f9b5c99b54c01d2b3622bc7f0c1fc987f0bf3e2b16cfd7e0d8cb59895068d76637996d75f92d55838bd8b18abe9d1848f644f1c7640f12a530f980d3646b8d7d6b9ad64983238dc6b8864db962cae1af68ae498fe9f0ef473412fb05b9d15797de42a44e4bd339905f7a3064792f563ff651ad2ffa3c4df66d897c1c457bb338ef92c97f0a2a1b8db2d33daa369b3417b4795455111f3ec272e491a7c6f60b02bf3689a85a5c34cc6e2bcb19c382cdfebb5c3b20f8d35907bff5e0bcc9686fac411de26cce4a9ef9a96a8ea50207443168eb44686d7cb7ca0a5d22ce50890c7d86e5c06c480746e0bccfc6415ea2941d210c528a05aa9c69c7b3263d3d9807cad31ccae0e432b4f097b3585078bd52f5c23ef4df648cc751b9a2fae892997b26c2e0406646472a6fd42aa41ef41a00212d4e6c06a04c777099c829262bbf83a37b1657cd9d6b8261882695c14c654a0ec0e01d28b19356c6ab39f06ec7e281d9352d901ce1c3d1e896a782a81a72ec1b17e386ca14bcfc04cac9edc34786c2a900e431bb38c0f98b2ce78bade88c96bfeafa200ea86672f774a39afc7a43c0eccbde9d376d29110f2472fa51aa30dbbd0214e310eab678c985de0f7c48930fe3f186d8601fcb00cf5c7df119b0121876524d7a2c13336b5e376e084a616c171607d0983c277c58a6dd6043b72d173ec8a45f031544d4779fdcd90d6efa850dc77ef4e5751d3eebcff4e8d70ea3a811e5c145c56c52dbd577b040f85e933a4ec5acaa71de561e2a92131f44455d810389634954a0cce765bc982315b1dcf4db1fa000836942a693e75c488e18eb1c169cf1e52491a66386718c7e3e0e6045cfdb749626d9de114f5e942d02ad9c9d5ff9ae9a227db23013e7c55bfb7d5688e34612398279c96c3eaaeab5985814f48c899c00b7f018b364f6614a5f74a5ca30147eb6dcfaf268242d431d9cf1169fa1ac4c3bdcb6d37e0").expect("decode failed"));

    // let payload: Vec<u8> = (0..2058).map(|_| rand::random()).collect();
    // println!("random paylod: {:x?}", &payload[0..128]);
    // packet.extend(payload);

    packet
}

pub fn create_quic_protected_payload_packet(
    payload_size: usize,
    destination_connection_id: &str,
) -> Vec<u8> {
    let mut packet = QuicShortHeader {
        destination_connection_id: destination_connection_id.to_string(),
        ..Default::default()
    }
    .to_vec();

    // pad with randomness.
    let padding_size = payload_size - packet.len();
    let payload: Vec<u8> = (0..padding_size).map(|_| rand::random()).collect();
    packet.extend(payload);

    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIENT_CONNECTION_ID: &str = "fce5816263165036";
    const SERVER_CONNECTION_ID: &str = "f25b987a5d73ccdf";

    #[test]
    fn long_header_prefix_size() {
        // Long header prefix size should be 7 plus the length of the destination ID plus the
        // length of the source ID. The destination and source IDs are hex strings, so the length
        // in bytes is half the length of the string.
        let destination_connection_id = String::from_iter(["0"; 40]);
        let source_connection_id = String::from_iter(["0"; 40]);
        let packet_prefix = QuicLongHeader {
            destination_connection_id,
            source_connection_id,
            ..Default::default()
        }
        .to_vec();
        assert_eq!(packet_prefix.len(), 47);

        let destination_connection_id = String::from_iter(["0"; 20]);
        let source_connection_id = String::from_iter(["0"; 20]);
        let packet_prefix = QuicLongHeader {
            destination_connection_id,
            source_connection_id,
            ..Default::default()
        }
        .to_vec();
        assert_eq!(packet_prefix.len(), 27);

        let packet_prefix = QuicLongHeader::default().to_vec();
        assert_eq!(packet_prefix.len(), 7);
    }

    #[test]
    fn initial_packet_partial_size() {
        let packet = QuicInitialPacketPartial {
            ..Default::default()
        };
        assert_eq!(packet.to_vec().len(), 8);
    }

    #[test]
    fn initial_packet_size() {
        let packet = create_quic_initial_packet(CLIENT_CONNECTION_ID);
        assert_eq!(packet.len(), 1200);
    }

    #[test]
    fn initial_packet_type() {
        let packet = create_quic_initial_packet(CLIENT_CONNECTION_ID);
        let first_byte = packet[0];
        let packet_type = first_byte & 0b0011_0000;
        assert_eq!(packet_type, 0b0000_0000);
    }

    #[test]
    fn handshake_packet_size() {
        let packet = create_quic_handshake_packet(CLIENT_CONNECTION_ID, SERVER_CONNECTION_ID, 0);
        assert_eq!(packet.len(), 1200);
    }

    #[test]
    fn handshake_packet_type() {
        let packet = create_quic_handshake_packet(CLIENT_CONNECTION_ID, SERVER_CONNECTION_ID, 0);
        let first_byte = packet[0];
        let packet_type = first_byte & 0b0011_0000;
        assert_eq!(packet_type, 0b0010_0000);
    }

    #[test]
    fn variable_length_integer() {
        let result = 0.to_variable_length_integer();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 0);

        // 1162 = 00000100_10001010
        let result = 1162.to_variable_length_integer();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 0b01000100);
        assert_eq!(result[1], 0b10001010);

        let result = 120.to_variable_length_integer();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 0b0100_0000);
        assert_eq!(result[1], 0b001111000);
    }

    #[test]
    #[should_panic]
    fn integer_too_large_for_variable_length_encoding() {
        usize::MAX.to_variable_length_integer();
    }

    // #[test]
    // fn create_quic_handshake_packet_fixed_bit() {
    //     // From the RFC (https://www.rfc-editor.org/rfc/rfc9000.html#name-long-header-packets):
    //     //      [The fixed bit] is set to 1, unless the packet is a Version Negotiation packet.
    //     //      Packets containing a zero value for this bit are not valid packets in this version
    //     //      and MUST be discarded.
    //     let packet = create_quic_handshake_packet("", "", 0);

    //     let first_byte = packet[0];

    //     assert_eq!(first_byte & 0b0100_0000, 0);
    // }

    #[test]
    fn test_get_source_connection_id_from_initial_packet() {
        let packet = create_quic_initial_packet(CLIENT_CONNECTION_ID);
        let source_connection_id = get_source_connection_id_from_initial_packet(&packet);

        assert_eq!(source_connection_id, CLIENT_CONNECTION_ID.to_string());
    }
}
