#[forbid(unsafe_code)]
use smallvec::{smallvec, SmallVec};

// for space and time efficiency, i needed to transform the arguments into a single byte
// this looks okay because the device control 1 ascii byte is obscure enough
const RESERVED_BYTE: u8 = 17;
const RESERVED_BYTE_INDEX: usize = RESERVED_BYTE as usize;
const ASTERISK_BYTE: u8 = 18;
const ASTERISK_BYTE_INDEX: usize = ASTERISK_BYTE as usize;

pub struct RouterBuilder {
    ascii_case_insensitive: bool,
    has_asterisk: bool,
}

impl RouterBuilder {
    pub fn new() -> RouterBuilder {
        RouterBuilder {
            ascii_case_insensitive: false,
            has_asterisk: false,
        }
    }
    fn replace<I, P>(&mut self, patterns: I) -> Vec<Vec<u8>>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<[u8]>,
    {
        let mut pts = vec![];

        for pattern in patterns.into_iter() {
            let mut cur = vec![];
            let bytes = pattern.as_ref();
            let mut in_brace = false;
            let mut should_check = true;
            let mut last_byte = 0;

            for b in bytes.iter() {
                last_byte = *b;
                if should_check {
                    in_brace = *b == b'{';
                }
                if in_brace {
                    should_check = false;
                    if *b == b'}' {
                        should_check = true;
                        cur.push(RESERVED_BYTE);
                    }
                    continue;
                }
                cur.push(*b);
            }
            if last_byte == b'*' {
                cur.pop();
                cur.push(ASTERISK_BYTE);
                self.has_asterisk = true;
            }
            pts.push(cur);
        }
        pts
    }
    pub fn build<I, P>(&mut self, patterns: I) -> Router
    where
        I: IntoIterator<Item = P>,
        P: AsRef<[u8]>,
    {
        let patterns = self.replace(patterns);
        let mut states = make_states();

        for (i, bytes) in patterns.into_iter().enumerate() {
            let mut state = &mut Default::default();
            let mut cur_states = &mut states;

            for ch in bytes.iter() {
                let ch = if self.ascii_case_insensitive {
                    ascii_to_lower(*ch)
                } else {
                    *ch
                };

                state = &mut cur_states[ch as usize];

                if state.trans.is_none() {
                    state.trans = Some(make_states());
                }
                cur_states = state.trans.as_mut().unwrap();
            }

            state.match_index = Some(i);
        }

        let casing = if self.ascii_case_insensitive {
            INSENSITIVE
        } else {
            SENSITIVE
        };

        Router {
            states,
            casing,
            has_asterisk: self.has_asterisk,
        }
    }

    pub fn ascii_case_insensitive(&mut self, yes: bool) -> &mut RouterBuilder {
        self.ascii_case_insensitive = yes;
        self
    }
}

const fn ascii_to_lower(b: u8) -> u8 {
    if b'A' <= b && b <= b'Z' {
        b.to_ascii_lowercase()
    } else {
        b
    }
}

const ARRAY_DEFAULT_SIZE: usize = 4;

#[derive(Eq, PartialEq, Debug)]
pub struct Array {
    array: SmallVec<[(usize, usize); ARRAY_DEFAULT_SIZE]>,
    i: usize,
}

impl Array {
    fn new() -> Self {
        let array: SmallVec<[(usize, usize); ARRAY_DEFAULT_SIZE]> =
            smallvec!((0usize, 0usize); ARRAY_DEFAULT_SIZE);
        Self { array, i: 0 }
    }

    pub fn to_vec(&self) -> Vec<(usize, usize)> {
        let mut v = self.array.to_vec();
        v.retain(|el| el.1 != 0);
        v
    }

    pub fn is_empty(&self) -> bool {
        self.array.is_empty()
    }

    fn push(&mut self, value: (usize, usize)) {
        if self.i < ARRAY_DEFAULT_SIZE {
            self.array[self.i] = value;
            self.i += 1;
            return;
        }

        self.array.push(value)
    }
}

/// Match represents a matched route
/// index is its position in the original collection
/// args is a vector of start and end positions within the matched
/// route that represents the path arguments
#[derive(Eq, PartialEq, Debug)]
pub struct Match {
    index: usize,
    args: Option<Array>,
    multi_segments: Option<Array>,
}

impl Match {
    #[inline(always)]
    pub fn get_index(&self) -> usize {
        self.index
    }
    #[inline(always)]
    pub fn get_args(&self) -> Option<&Array> {
        self.args.as_ref()
    }
    #[inline(always)]
    pub fn get_segments(&self) -> Option<&Array> {
        self.multi_segments.as_ref()
    }
}

pub struct Router {
    states: Box<[State; 256]>,
    casing: [u8; 256],
    has_asterisk: bool,
}

impl Router {
    pub fn route<P>(&self, r: P) -> Option<Match>
    where
        P: AsRef<[u8]>,
    {
        let bytes = r.as_ref();
        let mut state;
        let mut match_index = None;
        let mut cur_states = &*self.states;
        let mut i = 0;
        let mut args: Option<Array> = None;
        let mut multi_segments: Option<Array> = None;

        'outer: loop {
            if i == bytes.len() {
                break;
            }
            let byte = self.casing[bytes[i] as usize];

            if self.has_asterisk {
                if let Some(index) = cur_states[ASTERISK_BYTE_INDEX].match_index {
                    let ms = match multi_segments {
                        Some(ref mut ms) => Some(ms),
                        None => {
                            multi_segments = Some(Array::new());
                            multi_segments.as_mut()
                        }
                    }
                    .unwrap();

                    let mut start = i;

                    loop {
                        let last = i == bytes.len() - 1;
                        if bytes[i] == b'/' || last {
                            i += last as usize;
                            ms.push((start, i));
                            start = i + 1;
                        }
                        i += 1;
                        if i >= bytes.len() {
                            break;
                        }
                    }

                    return Some(Match {
                        index,
                        args,
                        multi_segments,
                    });
                }
            }

            state = &cur_states[byte as usize];

            if let Some(trans) = &state.trans {
                match_index = state.match_index;
                cur_states = trans;
                i += 1;
                continue;
            }
            let arg = &cur_states[RESERVED_BYTE_INDEX];

            if let Some(trans) = arg.trans.as_deref() {
                let start = i;
                match_index = arg.match_index;
                cur_states = trans;
                let mut end;
                loop {
                    end = i >= bytes.len();
                    if end || bytes[i] == b'/' {
                        break;
                    }
                    i += 1;
                }

                let a = match args {
                    Some(ref mut ms) => Some(ms),
                    None => {
                        args = Some(Array::new());
                        args.as_mut()
                    }
                }
                .unwrap();

                a.push((start, i));
                if end {
                    break 'outer;
                }
                continue;
            }
            return None;
        }

        match match_index {
            Some(index) => Some(Match {
                index,
                args,
                multi_segments,
            }),
            None => None,
        }
    }
}

#[derive(Default, Debug)]
struct State {
    trans: Option<Box<[State; 256]>>,
    match_index: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec_to_array(v: Vec<(usize, usize)>) -> Option<Array> {
        if v.is_empty() {
            return None;
        }
        let mut a = Array::new();
        for i in v {
            a.push(i);
        }
        Some(a)
    }

    #[test]
    fn test_thai() {
        let route = vec!["/สวัสดี/{ผรหัสผู้ใช้}/*", "/สวัสดีชาวโลก"];
        let router = RouterBuilder::new().build(route);

        let r_str = "/สวัสดี/ปีเตอร์/ผม/น/ที่นี่";
        let m = router.route(r_str);
        assert_eq!(
            Some(Match {
                index: 0,
                args: vec_to_array(vec![(20, 41)]),
                multi_segments: vec_to_array(vec![(42, 48), (49, 52), (53, 71)]),
            }),
            m
        );

        let m = router.route("/สวัสดีชาวโลก/ปีเตอร์/ผม/น/ที่นี่");
        assert_eq!(None, m);

        let m = router.route("/สวัสดีชาวโลก");
        assert_eq!(
            Some(Match {
                index: 1,
                args: vec_to_array(Default::default()),
                multi_segments: vec_to_array(Default::default()),
            }),
            m
        );
    }

    #[test]
    fn test_cyrillic() {
        let route = vec!["/здравей/{потребител_ид}/*", "/здравейсвят"];
        let router = RouterBuilder::new().build(route);

        let r_str = "/здравей/петър/аз/съм/тук";
        let m = router.route(r_str);

        assert_eq!(
            Some(Match {
                index: 0,
                args: vec_to_array(vec![(16, 26)]),
                multi_segments: vec_to_array(vec![(27, 31), (32, 38), (39, 45)]),
            }),
            m
        );

        let m = router.route("/здравейсвят/петър/аз/съм/тук");
        assert_eq!(None, m);

        let m = router.route("/здравейсвят");
        assert_eq!(
            Some(Match {
                index: 1,
                args: vec_to_array(Default::default()),
                multi_segments: vec_to_array(Default::default()),
            }),
            m
        );
    }

    #[test]
    fn test_array() {
        let mut a = Array::new();
        a.push((1, 1));
        a.push((1, 1));
        a.push((1, 1));
        a.push((1, 1));
        assert!(!a.array.spilled());

        let mut a = Array::new();
        a.push((1, 1));
        a.push((1, 1));
        a.push((1, 1));
        a.push((1, 1));
        a.push((1, 1));
        assert!(a.array.spilled());
    }

    #[test]
    fn test_multi_segment() {
        let route = vec!["/hello/{user_id}/*", "/helloworld"];
        let router = RouterBuilder::new().build(route);

        let m = router.route("/hello/petar/i/am/here");
        assert_eq!(
            Some(Match {
                index: 0,
                args: vec_to_array(vec![(7, 12)]),
                multi_segments: vec_to_array(vec![(13, 14), (15, 17), (18, 22)]),
            }),
            m
        );

        let m = router.route("/helloworld/petar/i/am/here");
        assert_eq!(None, m);

        let m = router.route("/helloworld");
        assert_eq!(
            Some(Match {
                index: 1,
                args: vec_to_array(Default::default()),
                multi_segments: vec_to_array(Default::default()),
            }),
            m
        );
    }

    #[test]
    fn test_not_ok() {
        let route = vec!["/hello/{user_id}", "/helloworld"];
        let router = RouterBuilder::new().build(route);
        let m = router.route("/hello*");
        assert_eq!(None, m);
        let m = router.route("/hello1");
        assert_eq!(None, m);
        let m = router.route("/hello*/");
        assert_eq!(None, m);
        let m = router.route("/*");
        assert_eq!(None, m);
        assert_eq!(None, router.route("/hEllO/wOrlD"));
    }

    #[test]
    fn test_ci_not_ok() {
        let route = vec!["/Hello/{user_id}", "/Helloworld"];
        let router = RouterBuilder::new().build(route);
        let m = router.route("/helloworld");
        assert_eq!(None, m);

        let m = router.route("/Helloworld");
        assert_eq!(
            Some(Match {
                index: 1,
                args: vec_to_array(Default::default()),
                multi_segments: vec_to_array(Default::default()),
            }),
            m
        );

        let route = vec!["/hello/{user_id}", "/helloworld"];
        let router = RouterBuilder::new().build(route);
        let m = router.route("/Helloworld");
        assert_eq!(None, m);
    }

    #[test]
    fn test_ci_ok() {
        let route = vec!["/hello/{user_id}", "/helloworld"];
        let router = RouterBuilder::new()
            .ascii_case_insensitive(true)
            .build(route);
        let m = router.route("/HelloWorld");
        assert_eq!(
            Some(Match {
                index: 1,
                args: vec_to_array(Default::default()),
                multi_segments: vec_to_array(Default::default()),
            }),
            m
        );
    }

    #[test]
    fn test_ok() {
        let route = vec![
            "/helloworld",
            "/hello/world",
            "/hello/{user_id}",
            "/hello/{user_id}/world",
            "/hello/world/{user_id}",
        ];
        let router = RouterBuilder::new().build(route);
        let m = router.route("/hello/*");
        assert_eq!(
            Some(Match {
                index: 2,
                args: vec_to_array(vec![(7, 8)]),
                multi_segments: vec_to_array(Default::default()),
            }),
            m
        );

        assert_eq!(
            Some(Match {
                index: 1,
                args: vec_to_array(Default::default()),
                multi_segments: vec_to_array(Default::default()),
            }),
            router.route("/hello/world")
        );

        assert_eq!(
            Some(Match {
                index: 2,
                args: vec_to_array(vec![(7, 12)]),
                multi_segments: vec_to_array(Default::default()),
            }),
            router.route("/hello/petar")
        );

        assert_eq!(
            Some(Match {
                index: 3,
                args: vec_to_array(vec![(7, 12)]),
                multi_segments: vec_to_array(Default::default()),
            }),
            router.route("/hello/petar/world")
        );
        assert_eq!(
            Some(Match {
                index: 4,
                args: vec_to_array(vec![(13, 18)]),
                multi_segments: vec_to_array(Default::default()),
            }),
            router.route(&"/hello/world/world")
        );

        assert_eq!(None, router.route(&"/hello/world/world/world"));
    }
}

const SENSITIVE: [u8; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97,
    98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135,
    136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154,
    155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173,
    174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192,
    193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211,
    212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230,
    231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249,
    250, 251, 252, 253, 254, 255,
];

const INSENSITIVE: [u8; 256] = [
    0,
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    8,
    9,
    10,
    11,
    12,
    13,
    14,
    15,
    16,
    17,
    18,
    19,
    20,
    21,
    22,
    23,
    24,
    25,
    26,
    27,
    28,
    29,
    30,
    31,
    32,
    33,
    34,
    35,
    36,
    37,
    38,
    39,
    40,
    41,
    42,
    43,
    44,
    45,
    46,
    47,
    48,
    49,
    50,
    51,
    52,
    53,
    54,
    55,
    56,
    57,
    58,
    59,
    60,
    61,
    62,
    63,
    64,
    ascii_to_lower(65),
    ascii_to_lower(66),
    ascii_to_lower(67),
    ascii_to_lower(68),
    ascii_to_lower(69),
    ascii_to_lower(70),
    ascii_to_lower(71),
    ascii_to_lower(72),
    ascii_to_lower(73),
    ascii_to_lower(74),
    ascii_to_lower(75),
    ascii_to_lower(76),
    ascii_to_lower(77),
    ascii_to_lower(78),
    ascii_to_lower(79),
    ascii_to_lower(80),
    ascii_to_lower(81),
    ascii_to_lower(82),
    ascii_to_lower(83),
    ascii_to_lower(84),
    ascii_to_lower(85),
    ascii_to_lower(86),
    ascii_to_lower(87),
    ascii_to_lower(88),
    ascii_to_lower(89),
    ascii_to_lower(90),
    91,
    92,
    93,
    94,
    95,
    96,
    97,
    98,
    99,
    100,
    101,
    102,
    103,
    104,
    105,
    106,
    107,
    108,
    109,
    110,
    111,
    112,
    113,
    114,
    115,
    116,
    117,
    118,
    119,
    120,
    121,
    122,
    123,
    124,
    125,
    126,
    127,
    128,
    129,
    130,
    131,
    132,
    133,
    134,
    135,
    136,
    137,
    138,
    139,
    140,
    141,
    142,
    143,
    144,
    145,
    146,
    147,
    148,
    149,
    150,
    151,
    152,
    153,
    154,
    155,
    156,
    157,
    158,
    159,
    160,
    161,
    162,
    163,
    164,
    165,
    166,
    167,
    168,
    169,
    170,
    171,
    172,
    173,
    174,
    175,
    176,
    177,
    178,
    179,
    180,
    181,
    182,
    183,
    184,
    185,
    186,
    187,
    188,
    189,
    190,
    191,
    192,
    193,
    194,
    195,
    196,
    197,
    198,
    199,
    200,
    201,
    202,
    203,
    204,
    205,
    206,
    207,
    208,
    209,
    210,
    211,
    212,
    213,
    214,
    215,
    216,
    217,
    218,
    219,
    220,
    221,
    222,
    223,
    224,
    225,
    226,
    227,
    228,
    229,
    230,
    231,
    232,
    233,
    234,
    235,
    236,
    237,
    238,
    239,
    240,
    241,
    242,
    243,
    244,
    245,
    246,
    247,
    248,
    249,
    250,
    251,
    252,
    253,
    254,
    255,
];

fn make_states() -> Box<[State; 256]> {
    let states = [
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
        State {
            trans: None,
            match_index: None,
        },
    ];

    Box::new(states)
}
