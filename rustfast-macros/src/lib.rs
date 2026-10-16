//! RustFast 派生宏
//!
//! `#[derive(Model)]` 为结构体自动实现 `rustfast::framework::database::model::Model`。
//!
//! ```ignore
//! #[derive(Model)]
//! #[model(table = "users", primary_key = "id")]  // 均可省略
//! pub struct User {
//!     pub id: String,
//!     pub name: String,
//!     #[model(skip)]                                // 不参与 from_row / to_values
//!     pub cached: String,
//!     #[model(skip_insert)]                         // 不参与 to_values（如 DB 侧生成的时间戳）
//!     pub created_at: String,
//! }
//! ```
//!
//! 默认规则：表名 = 结构体名 snake_case + "s"（User → users），主键 = "id"。

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitStr};

#[proc_macro_derive(Model, attributes(model))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_model(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_model(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "#[derive(Model)] does not support generic structs",
        ));
    }

    let name = &input.ident;

    // 结构体级属性：table / primary_key
    let mut table: Option<String> = None;
    let mut primary_key: Option<String> = None;
    for attr in &input.attrs {
        if !attr.path().is_ident("model") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("table") {
                let lit: LitStr = meta.value()?.parse()?;
                table = Some(lit.value());
                Ok(())
            } else if meta.path.is_ident("primary_key") {
                let lit: LitStr = meta.value()?.parse()?;
                primary_key = Some(lit.value());
                Ok(())
            } else {
                Err(meta.error("unsupported model attribute, expected `table` or `primary_key`"))
            }
        })?;
    }

    let table = table.unwrap_or_else(|| default_table_name(&name.to_string()));
    let primary_key = primary_key.unwrap_or_else(|| "id".to_string());

    // 字段（仅支持具名字段结构体）
    // #[model(skip)]        — 不参与 from_row / to_values
    // #[model(skip_insert)] — 不参与 to_values（如数据库自动生成的 created_at）
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => named
                .named
                .iter()
                .map(|f| {
                    let (skip, skip_insert) = field_flags(f);
                    (f.ident.clone().unwrap(), skip, skip_insert)
                })
                .collect::<Vec<_>>(),
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "#[derive(Model)] only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "#[derive(Model)] only supports structs",
            ))
        }
    };

    let row_fields: Vec<&syn::Ident> = fields
        .iter()
        .filter(|(_, skip, _)| !skip)
        .map(|(ident, _, _)| ident)
        .collect();
    let row_field_names: Vec<String> = row_fields.iter().map(|f| f.to_string()).collect();

    let skipped_fields: Vec<&syn::Ident> = fields
        .iter()
        .filter(|(_, skip, _)| *skip)
        .map(|(ident, _, _)| ident)
        .collect();

    let insert_fields: Vec<&syn::Ident> = fields
        .iter()
        .filter(|(_, skip, skip_insert)| !skip && !skip_insert)
        .map(|(ident, _, _)| ident)
        .collect();
    let insert_field_names: Vec<String> = insert_fields.iter().map(|f| f.to_string()).collect();

    let expanded = quote! {
        impl ::rustfast::framework::database::model::Model for #name {
            fn table_name() -> &'static str {
                #table
            }

            fn primary_key() -> &'static str {
                #primary_key
            }

            fn from_row(
                row: &::rustfast::framework::database::model::Row,
            ) -> ::rustfast::framework::error::Result<Self> {
                Ok(Self {
                    #( #row_fields: row.get_or(#row_field_names), )*
                    // skip 字段不在数据库中，以类型默认值初始化
                    #( #skipped_fields: ::std::default::Default::default(), )*
                })
            }

            fn to_values(&self) -> Vec<(&'static str, ::serde_json::Value)> {
                vec![
                    #( (#insert_field_names, ::serde_json::json!(self.#insert_fields.clone())), )*
                ]
            }
        }
    };

    Ok(expanded)
}

/// 字段属性：(skip, skip_insert)
fn field_flags(field: &syn::Field) -> (bool, bool) {
    let mut skip = false;
    let mut skip_insert = false;
    for attr in &field.attrs {
        if !attr.path().is_ident("model") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                skip = true;
            } else if meta.path.is_ident("skip_insert") {
                skip_insert = true;
            }
            Ok(())
        });
    }
    (skip, skip_insert)
}

/// 默认表名：结构体名转 snake_case 后加 "s"（User → users，OrderItem → order_items）
fn default_table_name(ident: &str) -> String {
    let mut snake = String::new();
    for (i, ch) in ident.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                snake.push('_');
            }
            snake.push(ch.to_lowercase().next().unwrap());
        } else {
            snake.push(ch);
        }
    }
    format!("{}s", snake)
}
