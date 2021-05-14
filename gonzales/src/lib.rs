pub trait Route<T> {
    fn is_match(req: &Vec<&str>, method: &str) -> bool;
    fn get_tuple_args(req: &Vec<&str>) -> T;
    fn len() -> usize;
}

pub struct RouterBuilder {
    ascii_case_insensitive: bool,
}

impl RouterBuilder {
    pub fn new() -> RouterBuilder {
        RouterBuilder {
            ascii_case_insensitive: false,
        }
    }
    pub fn build<I, P>(&self, patterns: I) -> Router
    where
        I: IntoIterator<Item = P>,
        P: AsRef<[u8]>,
    {
        let mut states = make_states();
        let mut max_arg_n = 0;
        for (i, pat) in patterns.into_iter().enumerate() {
            let bytes = pat.as_ref();

            let mut state = &mut Default::default();
            let mut cur_states = &mut states;
            let mut cur_arg_n = 0;
            for ch in bytes.iter() {
                if *ch == '{' as u8 || *ch == '}' as u8 {
                    cur_arg_n += 1;
                }
                state = cur_states.0[*ch as usize].as_mut();
                if state.trans.is_none() {
                    state.trans = Some(make_states());
                }
                cur_states = state.trans.as_mut().unwrap();
            }

            cur_arg_n /= 2;

            if cur_arg_n > max_arg_n {
                max_arg_n = cur_arg_n;
            }

            state.match_index = Some(i);

            if self.ascii_case_insensitive {
                let mut state = &mut Default::default();
                let mut cur_states = &mut states;
                for ch in bytes.iter() {
                    let ch = opposite_ascii_case(*ch);
                    state = cur_states.0[ch as usize].as_mut();
                    if state.trans.is_none() {
                        state.trans = Some(make_states());
                    }
                    cur_states = state.trans.as_mut().unwrap();
                }
                state.match_index = Some(i);
            }
        }

        Router { states, max_arg_n }
    }

    pub fn ascii_case_insensitive(&mut self, yes: bool) -> &mut RouterBuilder {
        self.ascii_case_insensitive = yes;
        self
    }
}

fn opposite_ascii_case(b: u8) -> u8 {
    if b'A' <= b && b <= b'Z' {
        b.to_ascii_lowercase()
    } else if b'a' <= b && b <= b'z' {
        b.to_ascii_uppercase()
    } else {
        b
    }
}

pub struct Router {
    states: States,
    max_arg_n: usize,
}

#[derive(Debug)]
#[allow(unused)]
pub struct Match<'a> {
    index: usize,
    args: Vec<&'a [u8]>,
}

impl<'a> Match<'a> {
    #[inline(always)]
    pub fn get_index(&self) -> usize {
        self.index
    }
    #[inline(always)]
    pub fn get_args(&self) -> &Vec<&'a [u8]> {
        &self.args
    }
}

impl Router {
    pub fn route<'a, P>(&'a self, r: &'a P) -> Option<Match>
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
            let byte = bytes[i];
            state = &cur_states.0[byte as usize];

            if let Some(trans) = &state.trans {
                match_index = state.match_index;
                cur_states = trans;
                i += 1;
                continue;
            }

            let arg = &cur_states.0['*' as usize];

            if let Some(trans) = &arg.trans {
                if args.is_empty() {
                    args = Vec::with_capacity(self.max_arg_n);
                }
                let start = i;
                match_index = arg.match_index;
                cur_states = trans;
                while i < bytes.len() && bytes[i] != '/' as u8 {
                    i += 1;
                }
                args.push(&bytes[start..i]);
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
