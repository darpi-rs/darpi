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
}

impl RouterBuilder {
    pub fn new() -> RouterBuilder {
        RouterBuilder {
            ascii_case_insensitive: false,
        }
    }
    fn replace<I, P>(&self, patterns: I) -> Vec<Vec<u8>>
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
            }
            pts.push(cur);
        }
        pts
    }
    pub fn build<I, P>(&self, patterns: I) -> Router
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
                let ch = ascii_to_lower(*ch);
                state = cur_states.0[ch as usize].as_mut();

                if state.trans.is_none() {
                    state.trans = Some(make_states());
                }
                cur_states = state.trans.as_mut().unwrap();
            }

            state.match_index = Some(i);
        }

        Router {
            states,
            ascii_case_insensitive: self.ascii_case_insensitive,
        }
    }

    pub fn ascii_case_insensitive(&mut self, yes: bool) -> &mut RouterBuilder {
        self.ascii_case_insensitive = yes;
        self
    }
}

fn ascii_to_lower(b: u8) -> u8 {
    if b'A' <= b && b <= b'Z' {
        b.to_ascii_lowercase()
    } else {
        b
    }
}

pub struct Router {
    states: States,
    ascii_case_insensitive: bool,
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

    pub fn get(&self, i: usize) -> Option<(usize, usize)> {
        self.array.get(i).cloned()
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

        self.push(value)
    }
}

/// Match represents a matched route
/// index is its position in the original collection
/// args is a vector of start and end positions within the matched
/// route that represents the path arguments
#[derive(Eq, PartialEq, Debug)]
pub struct Match {
    index: usize,
    args: Array,
    multi_segments: Array,
}

impl Match {
    #[inline(always)]
    pub fn get_index(&self) -> usize {
        self.index
    }
    #[inline(always)]
    pub fn get_args(&self) -> &Array {
        &self.args
    }
    #[inline(always)]
    pub fn get_segments(&self) -> &Array {
        &self.multi_segments
    }
}

impl Router {
    #[inline(always)]
    fn get_byte(b: u8) -> u8 {
        b
    }
    #[inline(always)]
    fn get_ascii_lower_byte(b: u8) -> u8 {
        ascii_to_lower(b)
    }
    pub fn route<P>(&self, r: P) -> Option<Match>
    where
        P: AsRef<[u8]>,
    {
        let bytes = r.as_ref();
        let mut state;
        let mut match_index = None;
        let mut cur_states = &self.states;
        let mut i = 0;
        let mut args = Array::new();
        let mut multi_segments = Array::new();

        let case_fn = match self.ascii_case_insensitive {
            true => Self::get_ascii_lower_byte,
            false => Self::get_byte,
        };

        'outer: loop {
            if i == bytes.len() {
                break;
            }
            let byte = case_fn(bytes[i]);

            if let Some(index) = cur_states.0[ASTERISK_BYTE_INDEX].match_index {
                let mut start = i;

                loop {
                    let last = i == bytes.len() - 1;
                    if bytes[i] == b'/' || last {
                        i += last as usize;
                        multi_segments.push((start, i));
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

            state = &cur_states.0[byte as usize];

            if let Some(trans) = &state.trans {
                match_index = state.match_index;
                cur_states = trans;
                i += 1;
                continue;
            }
            let arg = &cur_states.0[RESERVED_BYTE_INDEX];

            if let Some(trans) = &arg.trans {
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
                args.push((start, i));
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
    trans: Option<States>,
    match_index: Option<usize>,
}

#[derive(Debug)]
struct States(pub [Box<State>; 256]);

#[cfg(test)]
mod tests {
    use super::*;

    fn vec_to_array(v: Vec<(usize, usize)>) -> Array {
        let mut a = Array::new();
        for i in v {
            a.push(i);
        }
        a
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

fn make_states() -> States {
    States([
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
        Box::new(State {
            trans: None,
            match_index: None,
        }),
    ])
}
