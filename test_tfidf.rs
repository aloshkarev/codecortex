use std::collections::HashMap;

fn main() {
    let terms = vec!["a".to_string(), "b".to_string(), "a".to_string()];
    let mut tf: HashMap<String, f64> = HashMap::new();
    for term in &terms {
        if let Some(count) = tf.get_mut(term) {
            *count += 1.0;
        } else {
            tf.insert(term.clone(), 1.0);
        }
    }
    println!("{:?}", tf);
}
