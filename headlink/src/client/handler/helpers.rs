

pub fn union_pub_key(a: &str, b: &str) -> (String, bool) {
    match a < b {
        true => {
            let mut a = a.to_string();
            a.push_str(b);
            (a, true)
        }
        false => {
            let mut b = b.to_string();
            b.push_str(a);
            (b, false)
        }
    }
}
