#![allow(unused)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use std::collections::HashMap;
use std::iter::zip;
use std::ops::Deref;
use std::sync::Once;
use quote::quote;
use syn::{self, Data, DataStruct, Fields, Type};
use syn::DeriveInput;

static ONCE: Once = Once::new();

static mut TYPES_MAP: Option<HashMap<&str, String>> = None;

fn init_types_map() {
    let mut map = HashMap::new();
    map.insert("i32", "INTEGER".to_string());
    map.insert("usize", "INTEGER".to_string());
    map.insert("u32", "INTEGER".to_string());
    map.insert("f64", "FLOAT".to_string());
    map.insert("String", "TEXT".to_string());
    map.insert("bool", "BOOLEAN".to_string());

    unsafe {
        TYPES_MAP = Some(map);
    }
}

pub(crate) fn get_types_map() -> &'static HashMap<&'static str, String> {
    ONCE.call_once(init_types_map);

    unsafe {
        TYPES_MAP.as_ref().unwrap()
    }
}

#[proc_macro_derive(Entity, attributes(table))]
pub fn my_default(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let id = ast.ident;

    let attribute = ast.attrs.iter().filter(
        |a| a.path().segments.len() == 1 && a.path().segments[0].ident == "table"
    ).nth(0).expect("table attribute required for deriving Entity!");

    let table: Ident = attribute.parse_args().unwrap();

    let Data::Struct(s) = ast.data else {
        panic!("Entity derive macro must use in struct");
    };

    check_id(&s);

    let types_map = get_types_map();
    let mut fields_map = get_fields_map(&s, types_map);

    let fields: Vec<&str> = fields_map.iter().map(|(k, v)| k.as_str()).collect();
    let param_index: Vec<String> = (1..=fields_map.len()).map(|i| format!("?{}", i)).collect();
    let insert_sql = format!("INSERT INTO {} ({}) VALUES ({})", table, fields.join(", "), param_index.join(", "));


    let update: Vec<String> = zip(fields.iter().filter(|x| x.deref().deref() != "id"), &param_index[..param_index.len() - 1])
                                .map(|(k, i)| format!("{}={}", k, i)).collect();

    let update_sql = format!("UPDATE {} SET {} WHERE id=?{}", table, update.join(", "), param_index.len());

    let delete_sql = format!("DELETE FROM {} WHERE id=?1", table);

    let fields_ident: Vec<Ident> = fields.iter().map(|f| Ident::new(f, Span::call_site())).collect();
    let field_index: Vec<usize> = (0..fields.len()).collect();
    let fields_without_id: Vec<Ident> = fields.iter().filter(|f| f.deref().deref() != "id").map(|f| Ident::new(f, Span::call_site())).collect();

    let select_sql = format!("SELECT {} FROM {}", fields.join(", "), table);
    let gen = quote! {
        impl Entity for #id {
            fn persist(&self) {
                database().execute(#insert_sql, (#(&self.#fields_ident), *));
            }

            fn delete(&self) {
                database().execute(#delete_sql, (&self.id, ));
            }

            fn update(&self) {
                database().execute(#update_sql, (#(&self.#fields_without_id), *, &self.id));
            }

            fn find<P>(query: &str, params: P) -> Result<Vec<Self>, Error> where P: Params, Self: Sized{
                let mut stmt = database().prepare(&format!("{} WHERE {}", #select_sql, query))?;
                let mut result = Vec::new();
                let mut rows = stmt.query(params)?;
                while let Some(row) = rows.next()? {
                    let p = Self {
                        #(#fields_ident: row.get(#field_index)?,)*
                    };
                    result.push(p);
                };

                Result::Ok(result)
            }
        }
    };
    gen.into()
}

fn check_id(s: &DataStruct) {
    let has_id = s.fields.iter().any(|f| {
        if let Some(ref field) = f.ident {
            field.to_string() == "id" // type? who cares
        } else {
            false
        }
    });

    if !has_id {
        panic!("Entity struct must have `id` field");
    }
}

fn get_fields_map(s: &DataStruct, types_map: &HashMap<&str, String>) -> Vec<(String, String)> {
    let mut fields_map = vec![];
    if let Fields::Named(fields) = &s.fields {
        for field in &fields.named {
            if let Some(field_name) = &field.ident {
                if let Type::Path(type_path) = &field.ty {
                    if let Some(segment) = type_path.path.segments.last() {
                        let name = field_name.to_string();
                        let sql_type = types_map.get(&segment.ident.to_string() as &str).unwrap();
                        if name == "id" {
                            fields_map.push((name, format!("{} {}", sql_type, "PRIMARY KEY")));
                        } else {
                            fields_map.push((name, sql_type.to_string()));
                        }
                    }
                }
            }
        }
    }
    fields_map
}