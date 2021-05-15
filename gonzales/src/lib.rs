// for space and time efficiency, i needed to transform the arguments into a single byte
// this looks okay because the device control 1 ascii byte is obscure enough
const RESERVED_BYTE: u8 = 17;
const RESERVED_BYTE_INDEX: usize = RESERVED_BYTE as usize;

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

            for b in bytes.iter() {
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
        let mut max_arg_n = 0;

        for (i, bytes) in patterns.into_iter().enumerate() {
            let mut state = &mut Default::default();
            let mut cur_states = &mut states;
            let mut cur_arg_n = 0;
            for ch in bytes.iter() {
                let ch = ascii_to_lower(*ch);
                if ch == RESERVED_BYTE {
                    cur_arg_n += 1;
                }
                state = cur_states.0[ch as usize].as_mut();
                if state.trans.is_none() {
                    state.trans = Some(make_states());
                }
                cur_states = state.trans.as_mut().unwrap();
            }

            if cur_arg_n > max_arg_n {
                max_arg_n = cur_arg_n;
            }

            state.match_index = Some(i);
        }

        Router {
            states,
            max_arg_n,
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
    max_arg_n: usize,
    ascii_case_insensitive: bool,
}

/// Match represents a matched route
/// index is its position in the original collection
/// args is a vector of start and end positions within the matched
/// route that represents the path arguments
#[derive(Eq, PartialEq, Debug)]
pub struct Match {
    index: usize,
    args: Vec<(usize, usize)>,
}

impl Match {
    #[inline(always)]
    pub fn get_index(&self) -> usize {
        self.index
    }
    #[inline(always)]
    pub fn get_args(&self) -> &Vec<(usize, usize)> {
        &self.args
    }
}

impl Router {
    pub fn route<P>(&self, r: P) -> Option<Match>
    where
        P: AsRef<[u8]>,
    {
        let bytes = r.as_ref();
        let mut state;
        let mut match_index = None;
        let mut cur_states = &self.states;
        let mut i = 0;
        let mut args = Vec::new();

        while i < bytes.len() {
            let byte = match self.ascii_case_insensitive {
                true => ascii_to_lower(bytes[i]),
                false => bytes[i],
            };

            state = &cur_states.0[byte as usize];

            if let Some(trans) = &state.trans {
                match_index = state.match_index;
                cur_states = trans;
                i += 1;
                continue;
            }
            let arg = &cur_states.0[RESERVED_BYTE_INDEX];

            if let Some(trans) = &arg.trans {
                if args.is_empty() {
                    args = Vec::with_capacity(self.max_arg_n);
                }
                let start = i;
                match_index = arg.match_index;
                cur_states = trans;
                while i < bytes.len() && bytes[i] != b'/' {
                    i += 1;
                }
                args.push((start, i));
                continue;
            }
            return None;
        }

        match match_index {
            Some(index) => Some(Match { index, args }),
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
                args: vec![]
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
                args: vec![(7, 8)]
            }),
            m
        );

        assert_eq!(
            Some(Match {
                index: 1,
                args: vec![]
            }),
            router.route("/hello/world")
        );

        assert_eq!(
            Some(Match {
                index: 2,
                args: vec![(7, 12)]
            }),
            router.route("/hello/petar")
        );

        assert_eq!(
            Some(Match {
                index: 3,
                args: vec![(7, 12)]
            }),
            router.route("/hello/petar/world")
        );
        assert_eq!(
            Some(Match {
                index: 4,
                args: vec![(13, 18)]
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
