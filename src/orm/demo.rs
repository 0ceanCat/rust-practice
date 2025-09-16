
use rusqlite::{Params,Error, Result};
use syn;
use orm_macro_derive::Entity;
use crate::orm::core::{Entity, database};

#[derive(Debug, Entity)]
#[table(person)]
struct Person {
    id: i32,
    name: String,
}

impl Person {
    fn new(id: i32, name: String) -> Person {
        Person {
            id, name
        }
    }
}

fn main(){
    let mut p = Person::new(1, String::from("haha"));
    p.persist();
    println!("persist: {:?}", Person::find("name=:name", &[(":name", "haha")]));
    p.name = String::from("new_name");
    p.update();
    println!("update: {:?}", Person::find("name=:name", &[(":name", "haha")]));
    println!("update: {:?}", Person::find("name=:name", &[(":name", "new_name")]));
    p.delete();
    println!("delete: {:?}", Person::find("name=:name", &[(":name", "new_name")]));
}