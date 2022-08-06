extern crate xml;

use std::collections::HashMap;
use std::io::{BufReader};
use std::fs::File;
use std::time::Duration;

use crossterm::event::{read, Event, KeyCode, poll};
use crossterm::style::Stylize;
use rand::{seq::SliceRandom, thread_rng};
use xml::reader::{EventReader, XmlEvent};

fn main() {
    ctrlc::set_handler(move || {
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    let parser = load_file();
    let data_pieces = parse_data(parser);
    let questions = generate_questions(data_pieces);
    run_game(questions);
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

fn parse_data(parser: EventReader<BufReader<File>>) -> Vec<DataPiece> {
    //Parse Questions
    let mut data: Vec<DataPiece> = Vec::new();
    let mut data_type = String::new();
    let mut data_text = String::new();

    for e in parser {
        match e {
            Ok(e) => match e {
                XmlEvent::StartElement { name, .. } => {
                    data_type = name.local_name.clone();
                }
                XmlEvent::EndElement { name } => {
                    if data_type == name.local_name {
                        data.push(DataPiece {
                            data_type: data_type.clone(),
                            data: data_text.clone(),
                        })
                    }
                }
                XmlEvent::Characters(s) => {
                    data_text = s.clone();
                }
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

fn generate_questions(data_pieces: Vec<DataPiece>) -> Vec<Question> {
    //Find the start and end indices of the question data
    let mut blocks: Vec<(usize, usize)> = Vec::new();

    let mut start_index = 0;
    for i in 0..data_pieces.len() {
        if i != start_index && &data_pieces[i].data_type == "prompt" {
            blocks.push((start_index, i));
            start_index = i;
        }
    }
    blocks.push((start_index, data_pieces.len()));

    //Generate the questions from the data pieces usign the start and end indices
    let mut questions: Vec<Question> = Vec::new();
    for block in blocks {
        let mut text: String = String::new();
        let mut answer: String = String::new();
        let mut wrong_answers: Vec<String> = Vec::new();

        for i in block.0..block.1 {
            let d = &data_pieces[i];
            match d.data_type.as_str() {
                "prompt" => text = d.data.clone(),
                "correctAnswer" => answer = d.data.clone(),
                "incorrectAnswers" => wrong_answers.push(d.data.clone()),
                _ => {
                    panic!("Unhandled property: {}", data_pieces[i].data_type)
                }
            }
        }

        questions.push(Question {
            text,
            answer,
            wrong_answers,
        });
    }

    questions
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

        let mut answer = usize::MAX;
        //Read the users response
        match read() {
            Ok(e) => match e {
                Event::Key(event) => {
                    //Check options for if that the one the user pressed
                    for option in options {
                        //Convert the option to a char
                        let option_char = char::from_digit((option + 1).try_into().unwrap(), 10)
                            .expect("Could not convert option to character.");
                        if event.code == KeyCode::Char(option_char) {
                            answer = option;
                        }
                    }
                }
                _ => {}
            },
            Err(_) => {
                println!("There was an error whilst reading the answer.")
            }
        }
        //Get rid of any pending events
        while poll(Duration::from_secs(0)).expect("Could not poll") {
            let _ = read();
        }

        //Show if they got it right or not
        if answer == correct_answer {
            println!("{}","Correct!".green());
            answered_correctly += 1;
        } else {
            println!("{}","Wrong!".red());
            answered_incorrectly += 1;
        }
        println!();
    }

    println!(
        "That's it! You answered {} questions correctly and {} incorrectly.",
        answered_correctly.to_string().green(), answered_incorrectly.to_string().red()
    );
}

#[derive(Debug)]
struct DataPiece {
    data_type: String,
    data: String,
}

#[derive(Clone, Debug)]
struct Question {
    text: String,
    answer: String,
    wrong_answers: Vec<String>,
}
