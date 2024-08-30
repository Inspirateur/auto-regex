use std::i32;
use difflib::sequencematcher::SequenceMatcher;
use lazy_static::lazy_static;
use regex::Regex;
use log::warn;

lazy_static! {
    static ref RE_TOK: Regex =
        Regex::new(r"(\\*\(\\+d\\*\+\\*\)|\\*\(\\*\.\\*\+\\*\)|[^\d\W]+|[0-9]+|\W)").unwrap();
    static ref RE_PUNCT: Regex = Regex::new(r"^[\s,._;:]$").unwrap();
    pub static ref RE_UNESC: Regex =
        Regex::new(r"\\([\\\.\+\*\?\(\)\|\[\]\{\}\^\$\#\&\-\~])").unwrap();
}

fn tokenize(text: &str) -> Vec<String> {
    RE_TOK
        .find_iter(text)
        .map(|m| m.as_str().to_lowercase())
        .collect()
}

fn regex_from_pair(sample1: &str, sample2: &str) -> Option<Regex> {
    let seq1 = tokenize(sample1);
    let seq2 = tokenize(sample2);
    let mut seq_matcher = SequenceMatcher::new(&seq1, &seq2);
    let (mut _i, mut _j, mut _n) = (0, 0, 0);
    let mut rule = String::new();
    let mut var1: String;
    let mut var2: String;
    let mut cst: String;
    for m in seq_matcher.get_matching_blocks() {
        var1 = seq1[(_i + _n)..m.first_start].join("");
        var2 = seq2[(_j + _n)..m.second_start].join("");
        cst = seq1[m.first_start..(m.first_start + m.size)].join("");
        if _n != 0 && m.size != 0 && (var1.len() == 0 || var2.len() == 0) {
            // there's no template
            return None;
        }
        let var_is_num = var1.parse::<u16>().is_ok() && var2.parse::<u16>().is_ok();
        if var_is_num || !RE_PUNCT.is_match(&cst) || m.size == 0 {
            if var1.len() > 0 {
                if var_is_num {
                    rule += r"(\d+)";
                } else {
                    rule += r"(.+)";
                }
            }
            rule += &regex::escape(&cst);
        }
        _i = m.first_start;
        _j = m.second_start;
        _n = m.size;
    }
    if rule == "(.+)" {
        None
    } else {
        Some(Regex::new(&format!("(?i)^{}$", rule)).unwrap())
    }
}

fn score_regex(matched: &Vec<String>, total_samples: usize, regex: &Regex) -> i32 {
    if matched.len() < 2 {
        return 0;
    }
    let matched_part = matched.len() as f32/total_samples as f32;
    let variable_part = regex.as_str().len() as f32/matched[0].len() as f32;
    ((matched_part*variable_part)*100.) as i32
}

fn first_regex(example: &str, samples: &Vec<String>) -> Option<Regex> {
    for sample in samples {
        let regex_opt = regex_from_pair(example, sample);
        if regex_opt.is_some() {
            return regex_opt;
        }
    }
    None
}

/// Tries to find a regex that best matches the provided example and the samples
/// The example may or may not be part of the sample list, it doesn't matter
/// Note: the resulting regex is case-insensitive (and lowercase)
pub fn extract_regex(example: String, mut samples: Vec<String>) -> Option<Regex> {
    let mut best_regex = None;
    let mut best_score = 0;
    let total_samples = samples.len();
    while let Some(new_regex) = first_regex(&example, &samples) {
        // move the matches to a separate Vec
        let mut matched = Vec::new();
        let mut i = 0;
        while i < samples.len() {
            if new_regex.is_match(&samples[i]) {
                matched.push(samples.remove(i));
            } else {
                i += 1;
            }
        }
        if matched.len() == 0 {
            // this should not happen but creates an infinite loop so we need to break out of it
            warn!(target: "auto-regex", "Regex: '{}' failed to match any samples", new_regex.as_str());
            samples.remove(0);
        }
        // score the new regex
        let new_score = score_regex(&matched, total_samples, &new_regex);
        if new_score > best_score {
            best_regex = Some(new_regex);
            best_score = new_score;
        }
    }
    best_regex
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use crate::extract_regex;

    fn assert_regex_correct(truth: Option<&str>, output: Option<Regex>) {
        assert_eq!(output.map(|r| r.as_str().to_string()), truth.map(|t| t.to_string()))
    }

    #[test]
    fn variable_digits() {
        let samples = vec![
            "[1080p] Episode S1E01.mkv".to_string(),
            "[1080p] Episode S1E02.mkv".to_string(),
            "[1080p] Episode S1E03.mkv".to_string(),
            "[1080p] Episode S1E10.mkv".to_string(),
        ];
        let output = extract_regex(samples[0].clone(), samples);
        assert_regex_correct(Some(r"(?i)^\[1080p\] episode s1e(\d+)\.mkv$"), output);
    }

    #[test]
    fn variable_text() {
        let samples = vec![
            "picture of a bird.png".to_string(),
            "picture of a dog.png".to_string(),
            "picture of a zebra.png".to_string(),
        ];
        let output = extract_regex(samples[0].clone(), samples);
        assert_regex_correct(Some(r"(?i)^picture of a (.+)\.png$"), output);
    }

    #[test]
    fn should_not_match() {
        let samples = vec![
            "fwip".to_string(),
            "clunk".to_string(),
            "augh".to_string(),
            "fffp".to_string(),
        ];
        let output = extract_regex(samples[0].clone(), samples);
        assert_regex_correct(None, output);
    }

    #[test]
    fn variable_case() {
        let samples = vec![
            "Item number 1.txt".to_string(),
            "item Number 2.txt".to_string(),
            "Item number 3.txt".to_string(),
        ];
        let output = extract_regex(samples[0].clone(), samples);
        assert_regex_correct(Some(r"(?i)^item number (\d+)\.txt$"), output);
    }

    #[test]
    fn noisy_list() {
        let samples = vec![
            "picture of a bird.png".to_string(),
            "picture of a dog.png".to_string(),
            "picture of a zebra.png".to_string(),
            "my wallpaper.png".to_string(),
            "auugh".to_string(),
        ];
        let output = extract_regex(samples[0].clone(), samples);
        assert_regex_correct(Some(r"(?i)^picture of a (.+)\.png$"), output);
    }

    #[test]
    fn multi_variable() {
        let samples = vec![
            "[1080p] Episode S1E01.mkv".to_string(),
            "[1080p] Episode S1E02.mkv".to_string(),
            "[1080p] Episode S1E03.mkv".to_string(),
            "[1080p] Episode S1E10.mkv".to_string(),
            "[1080p] Episode S2E01.mkv".to_string(),
            "[1080p] Episode S2E02.mkv".to_string(),
            "[1080p] Episode S2E03.mkv".to_string(),
            "[1080p] Episode S2E10.mkv".to_string(),
        ];
        let output = extract_regex(samples[0].clone(), samples);
        assert_regex_correct(Some(r"(?i)^\[1080p\] episode s(\d+)e(\d+)\.mkv$"), output);
    }

    #[test]
    fn final_boss() {
        let samples = vec![
            "[1080p] episode s1e01 - dog (chien).mkv".to_string(),
            "[1080p] Episode S1E02 - cat (chat).mkv".to_string(),
            "[1080P] Episode S1E03 - bird (oiseau).mkv".to_string(),
            "[1080p] Episode S1E10 - zebra (zèbre).mkv".to_string(),
            "Bonus Episode.mkv".to_string(),
        ];
        let output = extract_regex(samples[0].clone(), samples);
        assert_regex_correct(Some(r"(?i)^\[1080p\] episode s1e(\d+) \- (.+) \((.+)\)\.mkv$"), output);
    }
}