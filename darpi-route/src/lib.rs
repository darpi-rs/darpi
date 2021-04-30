pub trait Route<T> {
    fn is_match(req: &Vec<&str>, method: &str) -> bool;
    fn get_tuple_args(req: &Vec<&str>) -> T;
    fn len() -> usize;
}
