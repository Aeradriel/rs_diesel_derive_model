#![feature(extern_crate_item_prelude)]
#![recursion_limit = "128"]

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate proc_macro2;
extern crate regex;
extern crate syn;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{DeriveInput, Field, Ident, Type};

#[proc_macro_derive(Model)]
pub fn model(input: TokenStream) -> TokenStream {
    let string_input = input.clone().to_string();
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = ast.ident.clone();
    let fields: Vec<Field> = match ast.data {
        syn::Data::Enum(..) => panic!("#[findable_by] cannot be used with enums"),
        syn::Data::Union(..) => panic!("#[findable_by] cannot be used with unions"),
        syn::Data::Struct(ref body) => body.fields.iter().map(|f| f.clone()).collect(),
    };
    let id_field_string = get_id_field(string_input.clone());
    let id_field_matches: Vec<&Field> = fields
        .iter()
        .filter(|f| f.ident.clone().unwrap().to_string() == id_field_string)
        .collect();
    let mut id_field_type: String = "i16".to_string();

    if id_field_matches.len() > 0 {
        let field = id_field_matches[0];

        if let Type::Path(ref field_type) = field.ty {
            id_field_type = field_type.path.segments[0].ident.to_string();
        }
    }

    let table_name = Ident::new(&get_table_name(string_input.clone()), Span::call_site());
    let id_field = Ident::new(&id_field_string, Span::call_site());
    let id_field_type = Ident::new(&id_field_type, Span::call_site());

    let model_funcs = quote!{
        impl #name {
            fn find(id: & #id_field_type, conn: &::diesel::PgConnection) -> Result<Self, JsonErrors> {
                use crate::schema::#table_name::dsl::#id_field as col;

                let res = #table_name::table.filter(col.eq(id)).first(conn)?;

                Ok(res)
            }

            fn save(self, conn: &::diesel::PgConnection) -> Result<Self, JsonErrors> {
                let res = self.save_changes(conn)?;

                Ok(res)
            }

            fn all(conn: &::diesel::PgConnection) -> Result<Vec<Self>, JsonErrors> {
                let res = #table_name::table.load(conn)?;

                Ok(res)
            }

            fn destroy(self, conn: &::diesel::PgConnection) -> Result<(), JsonErrors> {
                use crate::schema::#table_name::dsl::#id_field as col;

                diesel::delete(#table_name::table).filter(col.eq(self.id)).execute(conn)?;
                Ok(())
            }
        }
    };

    model_funcs.into()
}

fn get_id_field(input: String) -> String {
    use regex::Regex;

    let re = Regex::new(r###"#\[model_id[\s_]?=[\s_]?"(.*)"\]"###).unwrap();
    let id_field_attr = input
        .lines()
        .skip_while(|line| !line.trim_left().starts_with("#[model_id"))
        .next()
        .unwrap_or("#[model_id = \"id\"]");

    if let Some(id_field) = re
        .captures(id_field_attr)
        .expect("Malformed model_id attribute")
        .get(1)
    {
        id_field.as_str().to_string()
    } else {
        panic!("Malformed model_id attribute");
    }
}

fn get_table_name(input: String) -> String {
    use regex::Regex;

    let re = Regex::new(r###"#\[table_name = "(.*)"\]"###).unwrap();
    let table_name_attr = input
        .lines()
        .skip_while(|line| !line.trim_left().starts_with("#[table_name ="))
        .next()
        .expect("Struct must be annotated with #[table_name = \"...\"]");

    if let Some(table_name) = re
        .captures(table_name_attr)
        .expect("Malformed table_name attribute")
        .get(1)
    {
        table_name.as_str().to_string()
    } else {
        panic!("Malformed table_name attribute");
    }
}
