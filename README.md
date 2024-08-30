# auto-regex
Rust crate to find a regex rules that best matches a list of string

### Example
```rust
use auto_regex::infer_regex;

fn main() {
    // This has very little interest in static code and is best used with user interaction
    let samples = vec![
        "john.doe@gmail.com".to_string(),
        "alice.smith@gmail.com".to_string(),
        "bob.harris@gmail.com".to_string(),
        // bad samples will be ignored
        "badsample".to_string(),
    ];
    let example = "firstname.lastname@gmail.com".to_string();
    let regex = infer_regex(example, samples).unwrap();
    // Prints '(?i)^(.+)\.(.+)@gmail\.com$'
    // a regex that will extract first and last name from a gmail
    println!("{}", regex.as_str());
}
```

### Use cases
Used in [Inspirateur/SimpleRenamer](https://github.com/Inspirateur/SimpleRenamer), a smart file renamer. 

It can generally be useful in applications to give Regex power to users who don't know Regex or don't want to bother with it.