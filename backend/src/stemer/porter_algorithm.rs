fn is_vowel(word: &[char], i: usize) -> bool {
    let c = word[i];
    match c {
        'a' | 'e' | 'i' | 'o' | 'u' => true,
        'y' => i > 0 && !is_vowel(word, i - 1),
        _ => false,
    }
}

fn measure(word: &[char]) -> usize {
    let mut m = 0;
    let mut prev_vowel = false;

    for i in 0..word.len() {
        let current_vowel = is_vowel(word, i);
        if prev_vowel && !current_vowel {
            m += 1;
        }
        prev_vowel = current_vowel;
    }

    m
}

fn has_vowel(word: &[char]) -> bool {
    word.iter().enumerate().any(|(i, _)| is_vowel(word, i))
}

fn ends_with_cvc(word: &[char]) -> bool {
    if word.len() < 3 {
        return false;
    }

    let i = word.len() - 3;
    let last_c = word[i + 2];
    !is_vowel(word, i) && is_vowel(word, i + 1) && !is_vowel(word, i + 2) &&
        !['w', 'x', 'y'].contains(&last_c)
}

fn replace_suffix(word: &mut Vec<char>, suffix: &str, replacement: &str) -> bool {
    let suffix_chars: Vec<char> = suffix.chars().collect();
    let replacement_chars: Vec<char> = replacement.chars().collect();

    if word.ends_with(&suffix_chars) {
        let new_len = word.len() - suffix_chars.len();
        word.truncate(new_len);
        word.extend(replacement_chars);
        true
    } else {
        false
    }
}

fn replace_suffix_condition<F>(word: &mut Vec<char>, suffix: &str, replacement: &str, condition: F) -> bool
where
    F: Fn(&[char]) -> bool,
{
    let suffix_chars: Vec<char> = suffix.chars().collect();
    if word.ends_with(&suffix_chars) {
        let stem = &word[..word.len() - suffix_chars.len()];
        if condition(stem) {
            replace_suffix(word, suffix, replacement);
            true
        } else {
            false
        }
    } else {
        false
    }
}

fn step_1a(word: &mut Vec<char>) {
    if replace_suffix(word, "sses", "ss") { return }
    if replace_suffix(word, "ies", "i") { return }
    if replace_suffix(word, "ss", "ss") { return }

    if word.ends_with(&['s']) {
        let stem = &word[..word.len() - 1];
        if has_vowel(stem) {
            word.pop();
        }
    }
}

fn step_1b(word: &mut Vec<char>) {
    if replace_suffix_condition(word, "eed", "ee", |stem| measure(stem) > 0) {
        return;
    }

    let mut modified = false;
    let original = word.clone();

    if replace_suffix(word, "ed", "") && has_vowel(word) {
        modified = true;
    } else {
        *word = original.clone();
    }

    if !modified && replace_suffix(word, "ing", "") && has_vowel(word) {
        modified = true;
    } else if !modified {
        *word = original;
    }

    if modified {
        if replace_suffix(word, "at", "ate") ||
            replace_suffix(word, "bl", "ble") ||
            replace_suffix(word, "iz", "ize") {
            return;
        }

        if word.len() >= 2 {
            let last = word[word.len() - 1];
            let prev = word[word.len() - 2];
            if last == prev && !is_vowel(word, word.len() - 1) && !['l', 's', 'z'].contains(&last) {
                word.pop();
                return;
            }
        }

        if measure(word) == 1 && ends_with_cvc(word) {
            word.push('e');
        }
    }
}

fn step_1c(word: &mut Vec<char>) {
    if word.ends_with(&['y']) && has_vowel(&word[..word.len() - 1]) {
        word.pop();
        word.push('i');
    }
}

fn step_2(word: &mut Vec<char>) {
    let suffixes = [
        ("ational", "ate"), ("tional", "tion"), ("enci", "ence"),
        ("anci", "ance"), ("izer", "ize"), ("abli", "able"),
        ("alli", "al"), ("entli", "ent"), ("eli", "e"),
        ("ousli", "ous"), ("ization", "ize"), ("ation", "ate"),
        ("ator", "ate"), ("alism", "al"), ("iveness", "ive"),
        ("fulness", "ful"), ("ousness", "ous"), ("aliti", "al"),
        ("iviti", "ive"), ("biliti", "ble")
    ];

    for &(suffix, replacement) in &suffixes {
        if replace_suffix_condition(word, suffix, replacement, |stem| measure(stem) > 0) {
            return;
        }
    }
}

fn step_3(word: &mut Vec<char>) {
    let suffixes = [
        ("icate", "ic"), ("ative", ""), ("alize", "al"),
        ("iciti", "ic"), ("ical", "ic"), ("ful", ""),
        ("ness", "")
    ];

    for &(suffix, replacement) in &suffixes {
        if replace_suffix_condition(word, suffix, replacement, |stem| measure(stem) > 0) {
            return;
        }
    }
}

fn step_4(word: &mut Vec<char>) {
    let suffixes = [
        "al", "ance", "ence", "er", "ic", "able", "ible",
        "ant", "ement", "ment", "ent", "ou", "ism", "ate",
        "iti", "ous", "ive", "ize"
    ];

    for &suffix in &suffixes {
        if replace_suffix_condition(word, suffix, "", |stem| measure(stem) > 1) {
            return;
        }
    }

    if replace_suffix_condition(word, "ion", "", |stem| {
        stem.ends_with(&['s']) || stem.ends_with(&['t']) && measure(stem) > 1
    }) {}
}

fn step_5a(word: &mut Vec<char>) {
    if word.ends_with(&['e']) {
        let stem = &word[..word.len() - 1];
        let m = measure(stem);
        if m > 1 || (m == 1 && !ends_with_cvc(stem)) {
            word.pop();
        }
    }
}

fn step_5b(word: &mut Vec<char>) {
    if measure(word) > 1 && word.ends_with(&['l']) && word[word.len() - 2] == 'l' {
        word.pop();
    }
}

pub fn porter_stem(word: &str) -> String {
    let mut word = word.to_lowercase().chars().collect::<Vec<_>>();

    if word.len() <= 2 {
        return word.into_iter().collect();
    }

    step_1a(&mut word);
    step_1b(&mut word);
    step_1c(&mut word);
    step_2(&mut word);
    step_3(&mut word);
    step_4(&mut word);
    step_5a(&mut word);
    step_5b(&mut word);

    word.into_iter().collect()
}
