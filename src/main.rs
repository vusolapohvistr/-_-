use std::io::{BufRead};
use rand::seq::SliceRandom;
use sublime_fuzzy::{FuzzySearch, Scoring};


fn main() {
    let args: Vec<String> = std::env::args().collect();
    let chat_file_path = args.get(1);
    let dialog_core =
        if let Some(file_path) = chat_file_path {
            let file = std::fs::File::open(file_path).unwrap();
            let reader = std::io::BufReader::new(file);
            let json: serde_json::Value = serde_json::from_reader(reader).unwrap();

            let chat_history = json.as_object().unwrap();
            println!("{:?}", chat_history.get("name"));

            let parsed_messages: Vec<&serde_json::Value> = 
                chat_history
                    .get("messages")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|x | { x.get("type").unwrap().as_str().unwrap() == "message" })
                    .collect();
            println!("Поглинуто повідомлень {:?}", parsed_messages.len());

            let replies: Vec<Reply> = 
                parsed_messages
                    .iter()
                    .filter(|x| { match x.get("reply_to_message_id") {
                        Some(val) => val.is_number(),
                        None => false,
                    }})
                    .filter(|x| { match x.get("text") {
                        Some(val) => val.is_string(),
                        None => false,
                    }})
                    .map(|x| {
                        Reply {
                            text: x.get("text").unwrap().as_str().unwrap().into(),
                            reply_to_id: x.get("reply_to_message_id").unwrap().as_u64().unwrap() as usize,
                        }
                    })
                    .collect();
            println!("Відповідей {:?}", replies.len());

            let messages: Vec<Message> = 
                parsed_messages
                    .iter()
                    .filter(|x| { match x.get("text") {
                        Some(val) => val.is_string(),
                        None => false,
                    }})
                    .map(|x| {
                        Message {
                            id: x.get("id").unwrap().as_u64().unwrap() as usize,
                            text: x.get("text").unwrap().as_str().unwrap().into(),
                        }
                    })
                    .collect();
            DialogCore::new(messages, replies)
        } else {
            panic!("Write path to chat file!");
        };

    println!("Повідомлень для яких є відповіді {:?}", dialog_core.requests_responses.len());

    println!("Розпочнімо діалог");
    
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let response = dialog_core.get_response(&line.unwrap());
        println!("{}", response);
    }
}

struct Message {
    id: usize,
    text: String,
}

struct Reply {
    text: String,
    reply_to_id: usize,
}

struct RequestResponses {
    request: String,
    responses: Vec<String>,
}

struct DialogCore {
    requests_responses: Vec<RequestResponses>,
}

impl DialogCore {
    fn new(messages: Vec<Message>, replies: Vec<Reply>) -> Self {
        let mut replies_to_map = std::collections::BTreeMap::<usize, Vec<Reply>>::new();
        replies
            .into_iter()
            .for_each(|reply| {
                match replies_to_map.get_mut(&reply.reply_to_id) {
                    Some(replies) => { replies.push(reply); }
                    None => { replies_to_map.insert(reply.reply_to_id, vec![reply]); }
                };
            });
        
        let requests_responses: Vec<RequestResponses> = messages
            .into_iter()
            .map(|message| {
                RequestResponses {
                    request: message.text,
                    responses: match replies_to_map.remove(&message.id) {
                        Some(replies) => replies.into_iter().map(|x| { x.text }).collect(),
                        None => Vec::new(),
                    }
                }
            })
            .filter(|x| { !x.responses.is_empty() })
            .collect();
        
        DialogCore {
            requests_responses,
        }
    }

    fn get_response(&self, request: &str) -> String {      
        // Or pick from one of the provided `Scoring::...` methods like `emphasize_word_starts`
        let scoring = Scoring {
            bonus_consecutive: 128,
            bonus_word_start: 0,
            penalty_distance: 1,
            ..Scoring::default()
        };

        self.requests_responses
            .iter()
            .max_by_key(|x| {
                match FuzzySearch::new(request, &x.request).case_insensitive().score_with(&scoring).best_match() {
                    Some(sub_str) => sub_str.score(),
                    None => 0,
                }
            })
            .unwrap()
            .responses
            .choose(&mut rand::thread_rng())
            .unwrap()
            .to_owned()
    }
}