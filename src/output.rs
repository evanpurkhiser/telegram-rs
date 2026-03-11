use serde_json::Value;

pub fn print_output(value: &Value, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(value).unwrap());
    } else {
        println!("{}", toon::encode(value, None));
    }
}
