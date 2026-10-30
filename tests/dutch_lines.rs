//! The Dutch test set (#495): every real line and sentence in
//! `tests/fixtures/dutch/lines.json` reads as the file says it should. What the
//! file is, and how to add to it, is in the README beside it.

use kamosu::reading::read_line;
use kamosu::step_uses::step_uses;
use kamosu::units::{Measures, step_duration, step_temperature_in};
use serde_json::{Value, json};

fn recipes() -> Vec<Value> {
    let file = include_str!("fixtures/dutch/lines.json");
    let file: Value = serde_json::from_str(file).expect("the test set is JSON");
    file["recipes"].as_array().expect("recipes").clone()
}

fn array<'a>(value: &'a Value, field: &str) -> &'a Vec<Value> {
    value[field].as_array().expect("a list")
}

#[test]
fn every_line_reads_as_the_test_set_says() {
    let mut wrong = Vec::new();
    for recipe in recipes() {
        for line in array(&recipe, "lines") {
            let text = line["text"].as_str().expect("a line");
            let read = read_line(text, "nl").unwrap_or_default();
            let got = json!({
                "text": text,
                "amount": read.amount,
                "unit": read.unit,
                "food": read.target,
            });
            if got != *line {
                wrong.push(format!("{got}\n  should be {line}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_sentence_reads_as_the_test_set_says() {
    let mut wrong = Vec::new();
    for recipe in recipes() {
        let lines = array(&recipe, "lines");
        let ingredients: Vec<Value> = lines
            .iter()
            .map(|line| json!({ "kind": "ingredient", "text": line["text"] }))
            .collect();
        let readings: Vec<Value> = lines
            .iter()
            .map(|line| json!({ "target": line["food"] }))
            .collect();
        let sentences = array(&recipe, "steps");
        let steps: Vec<Value> = sentences
            .iter()
            .map(|step| json!({ "kind": "step", "text": step["text"] }))
            .collect();
        let title = recipe["title"].as_str().expect("a title");
        let uses = step_uses(&steps, &ingredients, &readings, Some("nl"), title);
        for (sentence, uses) in sentences.iter().zip(uses) {
            let text = sentence["text"].as_str().expect("a sentence");
            let oven = step_temperature_in("nl", text, Measures::Us, "en", None);
            let got = json!({
                "text": text,
                "uses": uses,
                "timer_seconds": step_duration(text, "nl"),
                "oven": oven.map(|oven| oven.measured),
            });
            if got != *sentence {
                wrong.push(format!("{got}\n  should be {sentence}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_test_set_is_a_few_hundred_lines_and_names_every_source() {
    let recipes = recipes();
    let lines: usize = recipes.iter().map(|r| array(r, "lines").len()).sum();
    let steps: usize = recipes.iter().map(|r| array(r, "steps").len()).sum();
    assert!(lines + steps >= 300, "{lines} lines and {steps} sentences");
    for recipe in &recipes {
        let source = recipe["source"].as_str().unwrap_or_default();
        assert!(source.starts_with("https://"), "{recipe}");
    }
}
