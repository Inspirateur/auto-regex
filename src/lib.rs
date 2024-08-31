use std::i32;
use difflib::sequencematcher::SequenceMatcher;
use lazy_static::lazy_static;
use regex::Regex;
use log::error;

lazy_static! {
    static ref RE_TOK: Regex =
        Regex::new(r"(\\*\(\\+d\\*\+\\*\)|\\*\(\\*\.\\*\+\\*\)|[^\d\W]+|[0-9]+|\W)").unwrap();
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
        if m.size > 0 {
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

fn score_regex(example: &str, regex: &Regex, matched: usize, total: usize) -> i32 {
    if matched < 2 {
        return 0;
    }
    let matched_part = matched as f32/total as f32;
    let variable_part = regex.as_str().len() as f32/example.len() as f32;
    ((matched_part*variable_part)*100.) as i32
}

fn first_new_regex(example: &str, samples: &Vec<String>, tried_mask: &mut [bool]) -> Option<Regex> {
    for (i, sample) in samples.iter().enumerate() {
        if tried_mask[i] {
            continue;
        }
        let regex_opt = regex_from_pair(example, sample);
        if regex_opt.is_some() {
            // this shouldn't be necessary but if the extracted regex does not match the sample 
            // it would create an infinite loop 
            tried_mask[i] = true;
            return regex_opt;
        }
    }
    None
}

/// Tries to find a regex that best matches the provided example and the samples
/// The example may or may not be part of the sample list, it doesn't matter
/// Note: the resulting regex is case-insensitive (and lowercase)
pub fn infer_regex(example: String, samples: Vec<String>) -> Option<Regex> {
    let mut best_regex = None;
    let mut best_score = 0;
    let total_samples = samples.len();
    let mut tried_mask = vec![false; samples.len()];
    while let Some(new_regex) = first_new_regex(&example, &samples, &mut tried_mask) {
        // count the matches and mark them as tried
        let mut matched = 0;
        for i in 0..samples.len() {
            if new_regex.is_match(&samples[i]) {
                matched += 1;
                tried_mask[i] = true;
            }
        }
        // score the new regex
        let new_score = score_regex(&example, &new_regex, matched, total_samples);
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

    use crate::infer_regex;

    fn assert_regex_correct(truth: Option<&str>, output: Option<Regex>) {
        assert_eq!(output.map(|r| r.as_str().to_string()), truth.map(|t| t.to_string()))
    }

    #[test]
    fn email() {
        let samples = vec![
            "john.doe@gmail.com".to_string(),
            "alice.smith@gmail.com".to_string(),
            "bob.harris@gmail.com".to_string(),
            "badsample".to_string(),
        ];
        let example = "firstname.lastname@gmail.com".to_string();
        let output = infer_regex(example, samples);
        assert_regex_correct(Some(r"(?i)^(.+)\.(.+)@gmail\.com$"), output);
    }

    #[test]
    fn variable_digits() {
        let samples = vec![
            "[1080p] Episode S1E01.mkv".to_string(),
            "[1080p] Episode S1E02.mkv".to_string(),
            "[1080p] Episode S1E03.mkv".to_string(),
            "[1080p] Episode S1E10.mkv".to_string(),
        ];
        let output = infer_regex(samples[0].clone(), samples);
        assert_regex_correct(Some(r"(?i)^\[1080p\] episode s1e(\d+)\.mkv$"), output);
    }

    #[test]
    fn variable_text() {
        let samples = vec![
            "picture of a bird.png".to_string(),
            "picture of a dog.png".to_string(),
            "picture of a zebra.png".to_string(),
        ];
        let output = infer_regex(samples[0].clone(), samples);
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
        let output = infer_regex(samples[0].clone(), samples);
        assert_regex_correct(None, output);
    }

    #[test]
    fn variable_case() {
        let samples = vec![
            "Item number 1.txt".to_string(),
            "item Number 2.txt".to_string(),
            "Item number 3.txt".to_string(),
        ];
        let output = infer_regex(samples[0].clone(), samples);
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
        let output = infer_regex(samples[0].clone(), samples);
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
        let output = infer_regex(samples[0].clone(), samples);
        assert_regex_correct(Some(r"(?i)^\[1080p\] episode s(\d+)e(\d+)\.mkv$"), output);
    }

    #[test]
    fn final_boss() {
        let samples = vec![
            "[1080p] episode s1e01 - dog (chien).mkv".to_string(),
            "[1080p] Episode S1E02 - cat (chat).mkv".to_string(),
            "[1080P] Episode S1E03 - bird (oiseau).mkv".to_string(),
            "[1080p] Episode S1E10 - zebra (zèbre).mkv".to_string(),
            "[1080p] Episode S2E01 - turtle (tortue).mkv".to_string(),
            "[1080p] Episode S2E02 - seahorse (hippocampe).mkv".to_string(),
            "[1080P] episode s2e03 - giraffe (giraffe).mkv".to_string(),
            "[1080p] Episode S2E10 - rabbit (lapin).mkv".to_string(),
            "Bonus Episode.mkv".to_string(),
        ];
        let output = infer_regex(samples[0].clone(), samples);
        assert_regex_correct(Some(r"(?i)^\[1080p\] episode s(\d+)e(\d+) \- (.+) \((.+)\)\.mkv$"), output);
    }
}