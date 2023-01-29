use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;

use crate::application::domain::{
    AnyValue, GetListResponseBody, GetOpenResponseBody, SmlListEntry, SmlMessageEnvelope,
    SmlMessages,
};

#[non_exhaustive]
#[derive(Debug)]
pub enum ParseError {
    Unknown,
}

pub type ParseResult<T> = Result<T, ParseError>;

/// Parse the body of an SML message (omitting header and footer)
pub fn parse_body(input: &[u8]) -> ParseResult<SmlMessages> {
    sml_parser::sml_body(input).map_err(|_| ParseError::Unknown)
}

/// Parse the whole SML message
pub fn parse_message(input: &[u8]) -> ParseResult<SmlMessages> {
    sml_parser::sml_messages(input).map_err(|_| ParseError::Unknown)
}

peg::parser! {
    grammar sml_parser<'a>() for [u8] {

        pub (crate) rule sml_body() -> SmlMessages =
            a: (sml_message_envelope())*
            {
                SmlMessages {
                    messages: a
                }
            }

        pub (crate) rule sml_messages() -> SmlMessages =
            header()
            a: (sml_message_envelope())*
            footer()
            {
                SmlMessages {
                    messages: a
                }
            }

        rule header() -> () =
            [0x1b] [0x1b] [0x1b] [0x1b] [0x01] [0x01] [0x01] [0x01]

        rule footer() -> () =
            [0x1b] [0x1b] [0x1b] [0x1b] [0x1a] [0..=255]*<3>

        rule sml_message_envelope() -> SmlMessageEnvelope =
            [0x76]
            transaction_id()
            group_no()
            abort_on_error()
            a:sml_message_body()
            crc()
            end_of_message()
            { a }

        rule crc() = [0x63] any_number() any_number()

        rule end_of_message() = [0x00]

        rule sml_message_body() -> SmlMessageEnvelope =
            get_open_response() /
            get_list_response() /
            get_close_response() // and more types

        rule get_open_response() -> SmlMessageEnvelope =
            [0x72] [0x63] [0x01] [0x01] [0x76]
            a: get_open_response_content()
            {
                SmlMessageEnvelope::GetOpenResponse(a)
            }

        rule get_open_response_content() -> GetOpenResponseBody =
            [0x01] [0x01]
            req_file_id: string()
            server_id: string()
            [0x01] [0x01]
            {
                GetOpenResponseBody {
                    server_id,
                    req_file_id
                }
            }

        rule get_close_response() -> SmlMessageEnvelope =
            [0x72] [0x63] [0x02] [0x01] [0x71]
            get_close_response_content()
            {
                SmlMessageEnvelope::GetCloseResponse
            }

        rule get_close_response_content() = [0x01]

        rule get_list_response() -> SmlMessageEnvelope =
            [0x72] [0x63] [0x07] [0x01] [0x77]
            a: get_list_response_content()
            {
                SmlMessageEnvelope::GetListResponse(a)
            }

        rule list_signature() = [0x01]

        rule act_gateway_time() = [0x01]*<0,1>

        rule get_list_response_content() -> GetListResponseBody =
            [0x01]
            server_id: string()
            list_name: string()
            obscure_prefix_in_get_list_response()
            value_list: list_sml_value() list_signature() act_gateway_time()
            {
                GetListResponseBody {
                    server_id,
                    list_name,
                    value_list
                }
            }

        rule obscure_prefix_in_get_list_response() =
            [0x72] [0x62] [0..=255] [0x65] [0..=255] [0..=255] [0..=255] [0..=255]

        rule list_sml_value() -> Vec<SmlListEntry> =
            prefix: [0x71..=0x7f]
            value: single_sml_value() * <{
                let length = prefix - 0x70;
                length as usize
            }>
            { value }

        rule single_sml_value() -> SmlListEntry =
            [0x77]
            object_name: string()
            status: optional_unsigned()
            value_time: string()
            unit: optional_unsigned()
            scaler: scaler()
            value: value() sml_value_signature()
            {
                SmlListEntry {
                    object_name, status, value_time, unit, scaler, value
                }
            }

        rule scaler() -> Option<isize> = optional_signed()

        rule value() -> AnyValue = arbitrary()

        rule sml_value_signature() = [0x01]

        rule arbitrary() -> AnyValue =
            (v:string() { AnyValue::String(v) }) /
            (v:signed() { AnyValue::Signed(v as isize) }) /
            (v:unsigned() { AnyValue::Unsigned(v as usize) })

        pub (crate) rule unsigned() -> usize =
            prefix: [0x62|0x63|0x64|0x65|0x66|0x67|0x68|0x69]
            value: (any_number()) * <{
                let length = prefix - 0x60;
                length as usize - 1
            }>
            {
                let left_padding = 8+1-(prefix - 0x60) as usize;
                let mut m = vec![0u8;left_padding];
                m.append(&mut value.to_vec());
                let mut rdr = Cursor::new(m);
                rdr.read_u64::<BigEndian>().unwrap() as usize
            }

        pub (crate) rule signed() -> isize =
            prefix: [0x52|0x53|0x54|0x55|0x56|0x57|0x58|0x59]
            value: (any_number()) * <{
                let length = prefix - 0x50;
                length as usize - 1
            }>
            {
                let left_padding = 8+1-(prefix - 0x50) as usize;
                let pad_byte = if value[0]>=128 { 0xFF } else { 0x00 };
                let mut m = vec![pad_byte;left_padding];
                m.append(&mut (value).to_vec());
                let mut rdr = Cursor::new(m);
                rdr.read_i64::<BigEndian>().unwrap() as isize
            }

        rule transaction_id() -> Vec<u8> =
            string()

        rule group_no() =
            [0x62] any_number()

        rule abort_on_error() =
            [0x62] [0x00]

        rule message_checksum() =
            any_number() any_number() any_number()

        rule any_number() -> u8 =
            [0..=255]

        rule optional_signed() -> Option<isize> =
            (v:signed() { Some(v) }) / ([0x01] { None })

        rule optional_unsigned() -> Option<usize> =
            (v:unsigned() { Some(v) }) / ([0x01] { None })

        rule string() -> Vec<u8> =
            short_string() / long_string()

        rule short_string() -> Vec<u8> =
            prefix: [0x01..=0x0f]
            value: (any_number()) * <{
                let length = prefix - 0x01;
                length as usize
            }>
            { value }

        rule long_string() -> Vec<u8> =
            prefix_1: [0x81..=0x83]
            prefix_2: [0x00..=0x0f]
            value: any_number() * <{
                let length = match prefix_1 {
                    0x81 => 14 + prefix_2,
                    0x82 => 30 + prefix_2,
                    0x83 => 46 + prefix_2,
                    _ => unreachable!()
                };
                length as usize
            }>
            { value }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    pub fn open() {
        //
        let example_open = vec![
            0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01, // header
            /* */ 0x76, // List with 6 entries
            /*      */ 0x05, 0x03, 0x2b, 0x18, 0x0f, // transactionId:
            /*      */ 0x62, 0x00, // groupNo:
            /*      */ 0x62, 0x00, //abortOnError:
            /*      */ 0x72, // messageBody: list with 2 entries
            /*          */ 0x63, 0x01, 0x01, // getOpenResponse:
            /*          */ 0x76, // list with 6 entries
            /*              */ 0x01, // codepage: no value
            /*              */ 0x01, // clientId: no value
            /*              */ 0x05, 0x04, 0x03, 0x02, 0x01, // reqFileId:
            /*              */ 0x0b, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
            0x0a, /*              */
            /*              */
            0x01, // refTime
            /*              */ 0x01, // smlVersion
            /*          */ 0x63, 0x49, 0x00, // CRC checksum of this message
            /*          */ 0x00, // end of this
            /* */ 0x1b, 0x1b, 0x1b, 0x1b, // Escape Sequenz
            /* */ 0x1a, 0x00, 0x70, 0xb2, // 1a + padding + CRC (2 bytes)
        ];

        let result = sml_parser::sml_messages(&example_open);

        assert_eq!(
            result,
            Ok(SmlMessages {
                messages: vec![SmlMessageEnvelope::GetOpenResponse(GetOpenResponseBody {
                    server_id: vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a],
                    req_file_id: vec![0x04, 0x03, 0x02, 0x01]
                })]
            })
        )
    }

    #[test]
    pub fn get_list_response_body() {
        //
        let example_list = vec![
            /* */ 0x76, //
            /*      */ 0x05, 0x01, 0xD3, 0xD7, 0xBB, //
            /*      */ 0x62, 0x00, //
            /*      */ 0x62, 0x00, //
            /*      */ 0x72, //
            /*          */ 0x63, 0x07, 0x01, // getListResponse
            /*          */ 0x77, //
            /*              */ 0x01, // clientId / optional
            /*              */ 0x0B, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
            0x0a, // serverId
            /*              */ 0x07, 0x01, 0x00, 0x62, 0x0A, 0xFF, 0xFF, // listName
            /*              */ 0x72, // actSensorTime / optional
            /*                  */ 0x62, 0x01, // choice: secIndex
            /*                  */ 0x65, 0x01, 0x8A, 0x4D, 0x15, // secIndex (uptime)
            /*              */ 0x72, // valList
            /*                  */ 0x77, // SML_ListEntry
            /*                      */ 0x07, 0x81, 0x81, 0xC7, 0x82, 0x03,
            0xFF, // objName
            /*                      */ 0x01, // status
            /*                      */ 0x01, // valTime
            /*                      */ 0x01, // unit
            /*                      */ 0x01, // scaler
            /*                      */ 0x04, 0x49, 0x53,
            0x4B, // value -- Herstelleridentifikation (ISK)
            /*                      */ 0x01, // valueSignature / optional
            /*                  */ 0x77, // SML_ListEntry
            /*                      */ 0x07, 0x01, 0x00, 0x01, 0x08, 0x00,
            0xFF, // objName
            /*                      */ 0x65, 0x00, 0x00, 0x01, 0x82, // status / optional
            /*                      */ 0x01, // valTime / optional
            /*                      */ 0x62, 0x1E, // unit / optional
            /*                      */ 0x52, 0xFF, // scaler / optional
            /*                      */ 0x59, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, // Gesamtverbrauch
            /*                      */ 0x01, // valueSignature / optional
            /*                  */ 0x01, // listSignature / optional
            /*                  */ 0x01, // actGatewayTime / optional
            /*      */ 0x63, 0xC6, 0x12, // crc
            /*      */ 0x00, // end of message
        ];

        let result = sml_parser::sml_body(&example_list);

        assert_eq!(
            result,
            Ok(SmlMessages {
                messages: vec![SmlMessageEnvelope::GetListResponse(GetListResponseBody {
                    server_id: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                    list_name: vec![1, 0, 98, 10, 255, 255],
                    value_list: vec![
                        SmlListEntry {
                            object_name: vec![129, 129, 199, 130, 3, 255],
                            status: None,
                            value_time: vec![],
                            unit: None,
                            scaler: None,
                            value: AnyValue::String(vec![73, 83, 75])
                        },
                        SmlListEntry {
                            object_name: vec![1, 0, 1, 8, 0, 255],
                            status: Some(386),
                            value_time: vec![],
                            unit: Some(30),
                            scaler: Some(-1),
                            value: AnyValue::Signed(0)
                        }
                    ]
                })]
            })
        )
    }

    #[test]
    pub fn get_close_response() {
        let example_close = vec![
            0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01, // header
            /*  */
            0x76, //
            /*      */ 0x05, 0x03, 0x2b, 0x18, 0x11, // transactionId:
            /*      */ 0x62, 0x00, // #groupNo:
            /*      */ 0x62, 0x00, // #abortOnError:
            /*      */ 0x72, //	messageBody:
            /*          */ 0x63, 0x02, 0x01, //	getCloseResponse:
            /*          */ 0x71, //
            /*              */ 0x01, // no value
            /*      */ 0x63, 0xfa, 0x36, // CRC
            /*      */ 0x00, //
            /* */ 0x1b, 0x1b, 0x1b, 0x1b, // escape sequence
            /* */ 0x1a, 0x00, 0x70, 0xb2, // 1a + padding + CRC (2 bytes)
        ];
        let result = sml_parser::sml_messages(&example_close);

        assert_eq!(
            result,
            Ok(SmlMessages {
                messages: vec![SmlMessageEnvelope::GetCloseResponse]
            })
        )
    }

    #[test]
    pub fn example_with_exotic_number_types() {
        let bytes = vec![
            0x1b, 0x1b, 0x1b, 0x1b, 0x01, 0x01, 0x01, 0x01, 0x76, 0x07, 0x00, 0x11, 0x06, 0x33,
            0x10, 0x11, 0x62, 0x00, 0x62, 0x00, 0x72, 0x63, 0x01, 0x01, 0x76, 0x01, 0x01, 0x07,
            0x00, 0x11, 0x04, 0x5d, 0x05, 0x5b, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x93, 0xa2, 0xc7, 0x01, 0x01, 0x63, 0xc0, 0xd3, 0x00, 0x76, 0x07, 0x00, 0x11, 0x06,
            0x33, 0x10, 0x12, 0x62, 0x00, 0x62, 0x00, 0x72, 0x63, 0x07, 0x01, 0x77, 0x01, 0x0b,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x93, 0xa2, 0xc7, 0x07, 0x01, 0x00, 0x62,
            0x0a, 0xff, 0xff, 0x72, 0x62, 0x01, 0x65, 0x04, 0x5d, 0x00, 0xd1, 0x79, 0x77, 0x07,
            0x81, 0x81, 0xc7, 0x82, 0x03, 0xff, 0x01, 0x01, 0x01, 0x01, 0x04, 0x45, 0x4d, 0x48,
            0x01, 0x77, 0x07, 0x01, 0x00, 0x00, 0x00, 0x09, 0xff, 0x01, 0x01, 0x01, 0x01, 0x0b,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x93, 0xa2, 0xc7, 0x01, 0x77, 0x07, 0x01,
            0x00, 0x01, 0x08, 0x00, 0xff, 0x64, 0x01, 0x02, 0x82, 0x01, 0x62, 0x1e, 0x52, 0x03,
            0x56, 0x00, 0x00, 0x00, 0x0e, 0x0d, 0x01, 0x77, 0x07, 0x01, 0x00, 0x02, 0x08, 0x00,
            0xff, 0x64, 0x01, 0x02, 0x82, 0x01, 0x62, 0x1e, 0x52, 0x03, 0x56, 0x00, 0x00, 0x00,
            0x14, 0xc1, 0x01, 0x77, 0x07, 0x01, 0x00, 0x01, 0x08, 0x01, 0xff, 0x01, 0x01, 0x62,
            0x1e, 0x52, 0x03, 0x56, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x77, 0x07, 0x01, 0x00,
            0x02, 0x08, 0x01, 0xff, 0x01, 0x01, 0x62, 0x1e, 0x52, 0x03, 0x56, 0x00, 0x00, 0x00,
            0x14, 0xc1, 0x01, 0x77, 0x07, 0x01, 0x00, 0x01, 0x08, 0x02, 0xff, 0x01, 0x01, 0x62,
            0x1e, 0x52, 0x03, 0x56, 0x00, 0x00, 0x00, 0x0e, 0x0d, 0x01, 0x77, 0x07, 0x01, 0x00,
            0x02, 0x08, 0x02, 0xff, 0x01, 0x01, 0x62, 0x1e, 0x52, 0x03, 0x56, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x77, 0x07, 0x81, 0x81, 0xc7, 0x82, 0x05, 0xff, 0x01, 0x01, 0x01,
            0x01, 0x83, 0x02, 0x65, 0xdc, 0xe7, 0x5e, 0xa7, 0x7a, 0xdf, 0x65, 0x1c, 0xc3, 0xc3,
            0xde, 0x43, 0xe2, 0xf6, 0xb2, 0x72, 0x0d, 0x78, 0x0b, 0xd2, 0xf0, 0x54, 0xa4, 0xc7,
            0x8c, 0xc3, 0x8c, 0xfc, 0x42, 0xb0, 0x6e, 0xa5, 0x27, 0xbf, 0xe0, 0xfc, 0x51, 0x4a,
            0xb8, 0x6f, 0x83, 0x03, 0x0f, 0x54, 0x1b, 0x4f, 0x87, 0x01, 0x01, 0x01, 0x63, 0xaa,
            0x28, 0x00, 0x76, 0x07, 0x00, 0x11, 0x06, 0x33, 0x10, 0x15, 0x62, 0x00, 0x62, 0x00,
            0x72, 0x63, 0x02, 0x01, 0x71, 0x01, 0x63, 0x0b, 0x74, 0x00, 0x1b, 0x1b, 0x1b, 0x1b,
            0x1a, 0x00, 0x0b, 0xc6,
        ];
        let result = sml_parser::sml_messages(&bytes);

        assert_eq!(result.is_ok(), true)
    }

    #[test]
    pub fn reads_8_bit_signed() {
        let bytes = vec![0x52, 0x02];

        let result = sml_parser::signed(&bytes);
        assert_eq!(result, Ok(2isize));
    }

    #[test]
    pub fn reads_negative_8_bit_signed() {
        let bytes = vec![0x52, 0xFE];

        let result = sml_parser::signed(&bytes);
        assert_eq!(result, Ok(-2isize));
    }

    #[test]
    pub fn reads_32_bit_signed() {
        let bytes = vec![0x55, 0x00, 0x00, 0x00, 0x01];

        let result = sml_parser::signed(&bytes);
        assert_eq!(result, Ok(1isize));
    }

    #[test]
    pub fn reads_negative_32_bit_signed() {
        let bytes = vec![0x55, 0xFF, 0xFF, 0xFF, 0xFF];

        let result = sml_parser::signed(&bytes);
        assert_eq!(result, Ok(-1isize));
    }

    #[test]
    pub fn reads_positive_56_bit_unsigned() {
        let bytes = vec![0x68, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];

        let result = sml_parser::unsigned(&bytes);
        assert_eq!(result, Ok(1usize));
    }

    #[test]
    pub fn reads_positive_56_bit_signed() {
        let bytes = vec![0x58, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];

        let result = sml_parser::signed(&bytes);
        assert_eq!(result, Ok(1isize));
    }

    #[test]
    pub fn reads_negative_56_bit_signed() {
        let bytes = vec![0x58, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];

        let result = sml_parser::signed(&bytes);
        assert_eq!(result, Ok(-1isize));
    }
}
