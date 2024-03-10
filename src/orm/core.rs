use std::sync::Once;
use rusqlite::{Connection, Error, Params};


pub(crate) trait Entity {
    fn persist(&self);

    fn delete(&self);

    fn update(&self);

    fn find<P>(query: &str, params: P) -> Result<Vec<Self>, Error> where P: Params, Self: Sized;
}


static mut DATABASE: Option<Connection> = None;
static ONCE: Once = Once::new();

fn init_singleton() {

    unsafe {
        DATABASE = Some(Connection::open("db").unwrap());
    }
}

pub(crate) fn database() -> &'static Connection {
    ONCE.call_once(init_singleton);

    unsafe {
        DATABASE.as_ref().unwrap()
    }
}
