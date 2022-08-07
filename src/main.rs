extern crate xml;

use std::fs::File;
use std::io::BufReader;

use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::style::Stylize;
use rand::{seq::SliceRandom, thread_rng};
use serde::Deserialize;
use xml::reader::{EventReader, XmlEvent};

fn main() {
    ctrlc::set_handler(move || {
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    let questions = get_questions();
    run_game(questions);
}

fn get_questions() -> Vec<Question> {
    println!("What question source should be used?");
    println!("1: File");
    println!("2: Web");

    loop {
        match read() {
            Ok(e) => match e {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('1'),
                    ..
                }) => {
                    return get_questions_from_file();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('2'),
                    ..
                }) => {
                    return get_questions_from_api();
                }
                _ => {
                    continue;
                }
            },
            Err(_) => {}
        }
    }
}

fn get_questions_from_api() -> Vec<Question> {
    let res = match reqwest::blocking::get("https://the-trivia-api.com/api/questions?limit=5") {
        Ok(res) => res,
        Err(_) => {
            println!("Error on download");
            std::process::exit(1)
        }
    };
    let questions: Vec<Question> = match res.json() {
        Ok(json) => json,
        Err(err) => {
            println!("Error on deserialiation: {err}");
            std::process::exit(1)
        }
    };

    questions
}

fn get_questions_from_file() -> Vec<Question> {
    let parser = load_file();
    let questions = parse_data(parser);
    questions
}

fn load_file() -> EventReader<BufReader<File>> {
    //Loading the file
    const FILENAME: &str = "questions.xml";
    let file = match File::open(FILENAME) {
        Ok(file) => file,
        Err(_) => {
            print!("{} not found. Exiting.", FILENAME);
            std::process::exit(1);
        }
    };
    //Create Buffer and parser
    let file = BufReader::new(file);

    EventReader::new(file)
}

fn parse_data(parser: EventReader<BufReader<File>>) -> Vec<Question> {
    //Parse Questions
    let mut data: Vec<Question> = Vec::new();
    let mut cur_question: Option<Question> = None;
    let mut cur_data: Option<String> = None;

    for e in parser {
        match e {
            Ok(e) => match e {
                XmlEvent::StartElement { name, .. } => match name.local_name.as_str() {
                    "question" => cur_question = Some(Question::new()),
                    "prompt" | "correctAnswer" | "incorrectAnswer" => match cur_question {
                        Some(_) => cur_data = Some(String::new()),
                        None => warn_unexpected_tag(name.local_name.as_str(), false),
                    },
                    _ => warn_unexpected_tag(name.local_name.as_str(), false),
                },
                XmlEvent::EndElement { name } => match name.local_name.as_str() {
                    "question" => match cur_question {
                        Some(_) => data.push(cur_question.take().unwrap()),
                        None => warn_unexpected_tag("question", true),
                    },
                    "prompt" | "correctAnswer" | "incorrectAnswer" => match cur_question {
                        Some(_) => {
                            let mut question = cur_question.take().unwrap();
                            let data = cur_data.take().unwrap();
                            if name.local_name == "prompt" {
                                question.text = data;
                            } else if name.local_name == "correctAnswer" {
                                question.answer = data;
                            } else if name.local_name == "incorrectAnswer" {
                                question.wrong_answers.push(data);
                            }
                            cur_question = Some(question)
                        }
                        None => warn_unexpected_tag(name.local_name.as_str(), true),
                    },
                    _ => {}
                },
                XmlEvent::Characters(s) => match cur_data {
                    Some(_) => {
                        let mut data = cur_data.take().unwrap();
                        data.push_str(s.as_str());
                        cur_data = Some(data);
                    }
                    None => {
                        panic!("We should not be getting characters here.")
                    }
                },
                _ => {}
            },
            Err(e) => {
                println!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    data
}

fn warn_unexpected_tag(name: &str, closing: bool) {
    if closing {
        println!("Unexpected closing {name} tag.")
    } else {
        println!("Unexpected {name} tag.")
    }
}

fn run_game(questions: Vec<Question>) {
    let mut rng = thread_rng();
    let mut answered_correctly = 0;
    let mut answered_incorrectly = 0;

    for q in questions {
        println!(" === {} ===", q.text);

        //Shuffle the order of the answers and display them
        let mut correct_answer = 0;
        let mut options: Vec<usize> = (0..q.wrong_answers.len() + 1).collect();
        options.shuffle(&mut rng);
        for (index, order) in options.iter().enumerate() {
            if order == &q.wrong_answers.len() {
                println!("{}: {}", index + 1, q.answer);
                correct_answer = index;
            } else {
                println!("{}: {}", index + 1, q.wrong_answers[order.clone()])
            }
        }

        let answer;
        //Read the users response
        'input: loop {
            match read() {
                Ok(e) => match e {
                    Event::Key(event) => {
                        //Check options for if that the one the user pressed
                        for option in options.clone() {
                            //Convert the option to a char
                            let option_char =
                                char::from_digit((option + 1).try_into().unwrap(), 10)
                                    .expect("Could not convert option to character.");
                            if event.code == KeyCode::Char(option_char) {
                                answer = option;
                                break 'input;
                            }
                        }
                    }
                    _ => {}
                },
                Err(_) => {
                    println!("There was an error whilst reading the answer.")
                }
            }
        }

        //Show if they got it right or not
        if answer == correct_answer {
            println!("{}", "Correct!".green());
            answered_correctly += 1;
        } else {
            println!("{} The correct answer is: {}", "Wrong!".red(), q.answer);
            answered_incorrectly += 1;
        }
        println!();
    }

    println!(
        "That's it! You answered {} questions correctly and {} incorrectly.",
        answered_correctly.to_string().green(),
        answered_incorrectly.to_string().red()
    );
}

#[derive(Clone, Debug, Deserialize)]
struct Question {
    #[serde(alias = "question")]
    text: String,
    #[serde(alias = "correctAnswer")]
    answer: String,
    #[serde(alias = "incorrectAnswers")]
    wrong_answers: Vec<String>,
}

impl Question {
    fn new() -> Question {
        Question {
            text: String::new(),
            answer: String::new(),
            wrong_answers: Vec::new(),
        }
    }
}
