use logos;
use logos::Logos;
use std::collections::HashMap;
use std::convert::TryFrom;

#[derive(Logos, Debug, PartialEq)]
pub enum ReqToken<'a> {
    // Tokens can be literal strings, of any length.
    #[token("/")]
    Slash,

    // Or regular expressions.
    #[regex("[a-zA-Z0-9[.]-_~!$&'()*+,;=:@]+")]
    PathSegment(&'a str),
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

#[derive(Debug)]
pub struct ReqRoute<'a> {
    pub values: Vec<ReqToken<'a>>,
}

impl<'a> TryFrom<&'a str> for ReqRoute<'a> {
    type Error = String;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        let mut lex = ReqToken::lexer(s);
        let mut values: Vec<ReqToken<'a>> = vec![];

        while let Some(next) = lex.next() {
            match next {
                ReqToken::Error => return Err("invalid ReqRoute".to_string()),
                _ => values.push(next),
            }
        }

        Ok(Self { values })
    }
}

#[derive(Logos, Debug, PartialEq)]
pub enum Token<'a> {
    // Tokens can be literal strings, of any length.
    #[token("/")]
    Slash,

    #[regex("[{]([^{}/]+)[}]")]
    Arg(&'a str),

    // Or regular expressions.
    #[regex(r"[a-zA-Z0-9[.]-_~!$&'()*+,;=:@]+")]
    PathSegment(&'a str),
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

#[derive(Debug)]
pub struct Route<'a> {
    pub values: Vec<Token<'a>>,
}

impl<'a> TryFrom<&'a str> for Route<'a> {
    type Error = String;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        let mut lex = Token::lexer(s);
        let mut values: Vec<Token<'a>> = vec![];

        while let Some(next) = lex.next() {
            match next {
                Token::Error => return Err("invalid route".to_string()),
                _ => values.push(next),
            }
        }

        Ok(Self { values })
    }
}

impl<'a> PartialEq<ReqRoute<'a>> for Route<'a> {
    fn eq(&self, other: &ReqRoute) -> bool {
        if self.values.len() != other.values.len() {
            return false;
        }

        for (i, tt) in self.values.iter().enumerate() {
            match (tt, &other.values[i]) {
                (Token::PathSegment(left), ReqToken::PathSegment(right)) => {
                    if left != right {
                        return false;
                    }
                }
                _ => {}
            }
        }

        true
    }
}

impl<'a> ReqRoute<'a> {
    pub fn extract_args(&self, route: &Route<'a>) -> Result<HashMap<&'a str, &'a str>, String> {
        if route != self {
            return Err("routes are not matching".to_string());
        }

        let mut args = HashMap::new();
        for (i, tok) in route.values.iter().enumerate() {
            match (tok, &self.values[i]) {
                (Token::Arg(key), ReqToken::PathSegment(value)) => {
                    args.insert(&key[1..key.len() - 1], *value);
                }
                _ => {}
            }
        }
        Ok(args)
    }
}

#[test]
fn route_to_string() {
    let def_route = Route::try_from("/user/id/{article}").unwrap();
    //let def_route = Route::try_from("/user/{id}/{article}").unwrap();
    let req_route = ReqRoute::try_from("/user/id/1").unwrap();
    assert_eq!(def_route, req_route);
    let args = req_route.extract_args(&def_route).unwrap();
    panic!("{:#?}", args);

    // asd("/user/{name}");
    // asd("/user/article");
    // let mut v = vec![
    //     "/user/{name}",
    //     "/user/article",
    //     "user/{id}/article",
    //     "user/id/article",
    //     "user/id/{article}",
    // ];
    // v.sort_by(|left, right| {
    //     let left_matches: Vec<usize> = left.match_indices('{').map(|t| t.0).collect();
    //     let left_count = left_matches.iter().fold(0, |acc, a| acc + a);
    //
    //     if left_matches.len() == 0 {
    //         return Ordering::Less;
    //     }
    //
    //     let right_matches: Vec<usize> = right.match_indices('{').map(|t| t.0).collect();
    //     let right_count = right_matches.iter().fold(0, |acc, a| acc + a);
    //
    //     if right_matches.len() == 0 {
    //         return Ordering::Greater;
    //     }
    //
    //     if left_matches.len() + left_count > right_matches.len() + right_count {
    //         return Ordering::Less;
    //     }
    //
    //     Ordering::Greater
    // });
    //println!("{:#?}", v);
}

impl<'a> ToString for Route<'a> {
    fn to_string(&self) -> String {
        let mut s = String::new();

        for tt in &self.values {
            let fragment = match tt {
                Token::Arg(s) => s,
                Token::PathSegment(s) => s,
                Token::Slash => "/",
                _ => "",
            };
            s.push_str(fragment);
        }
        s
    }
}

#[test]
fn are_equal() {
    let left: Route = Route::try_from("/user/{id}/{article}").unwrap();
    let right: ReqRoute = ReqRoute::try_from("/user/1/2").unwrap();
    assert_eq!(left, right);

    let args = right.extract_args(&left).unwrap();
    assert_eq!(args.len(), 2);
    assert_eq!(args.get("id"), Some(&"1"));
    assert_eq!(args.get("article"), Some(&"2"));

    let left: Route = Route::try_from("/user/{id}/article/{article}").unwrap();
    let right: ReqRoute = ReqRoute::try_from("/user/1/article/2").unwrap();
    assert_eq!(left, right);

    let args = right.extract_args(&left).unwrap();
    assert_eq!(args.len(), 2);
    assert_eq!(args.get("id"), Some(&"1"));
    assert_eq!(args.get("article"), Some(&"2"));

    let left: Route = Route::try_from("/user/{name}").unwrap();
    let right: ReqRoute = ReqRoute::try_from("/user/petar-asd").unwrap();
    assert_eq!(left, right);

    let args = right.extract_args(&left).unwrap();
    assert_eq!(args.len(), 1);
    assert_eq!(args.get("name"), Some(&"petar-asd"));
}

#[test]
fn are_not_equal() {
    let left: Route = Route::try_from("/user/{id}/article/{article}").unwrap();
    let right: ReqRoute = ReqRoute::try_from("/qwe-zxc").unwrap();
    assert_ne!(left, right);

    let args = right.extract_args(&left);
    assert_eq!(args, Err("routes are not matching".to_string()));
}
