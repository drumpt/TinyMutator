extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
use proc_macro::{TokenStream, TokenTree};
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use rand::seq::SliceRandom;
use std::{
    env, fs,
    io::prelude::*,
    io::{self, BufRead},
    path::Path,
    process::Command,
    str::FromStr,
    vec,
    cmp,
    collections::HashSet,
};
use syn::{
    parse_macro_input, parse_quote,
    spanned::Spanned,
    visit::{self, Visit},
    visit_mut::{self, VisitMut},
    DeriveInput, Expr, File, Item, ItemFn, Lit, LitInt, Result, Stmt, Type,
};

struct BinOpVisitor<'ast> {
    BinOps: Vec<&'ast mut syn::BinOp>,
    Line: usize,
    Column: usize,
    Covered: HashSet<usize>,
    Prevsize: usize,
}

impl<'ast> VisitMut for BinOpVisitor<'ast> {
    fn visit_bin_op_mut(&mut self, node: &mut syn::BinOp) {
        let start = node.span().start();
        let end = node.span().end();
        let mut isTarget = true;
        if !(start.line <= self.Line && self.Line <= end.line) {
            isTarget = false;
        }
        if let syn::BinOp::BitOr(or) = &node {
            if self.Covered.contains(&end.column) || self.Prevsize < self.Covered.len() {
                isTarget = false;
            } 
        } else {
            isTarget = false;
        }
        if isTarget {
            self.Covered.insert(end.column.clone());
            self.Column = end.column;
            *node = syn::BinOp::BitAnd(syn::token::And(node.span().clone()));
        } else {
            visit_mut::visit_bin_op_mut(self, node);

        }
    }
}

/** 
 * Get smallest list of lines which is parsable with ast
*/
pub fn find_min_parsable_lines(splitted_file: Vec<&str>, num_line: usize) -> (usize, usize) {
    for j in 1..cmp::max(splitted_file.len() - num_line, num_line - 0) { // length
        for i in 0..j {
            if num_line + i - j <= 0 || num_line + i > splitted_file.len() { continue; }
            // println!("{:#?}", &splitted_file[(num_line + i - j)..(num_line + i)].join("\t\r"));
            // println!("{} {}", num_line + i - j, num_line + i);
            match :: syn::parse_str::<Stmt>(&splitted_file[(num_line + i - j)..(num_line + i)].join("\t\r")) {
                Ok(stmt) => {
                    return (num_line + i - j, num_line + i);
                },
                Err(error) => {},
            }
        }
    }
    return (0, splitted_file.len());
}

/**
 * Modify specific line of given file
*/
pub fn mutate_file_by_line(file: String, num_line: usize) -> String {
    // println!("filename : {}", file);
    // println!("line : {}", num_line);
    let mut constants = vec!["0", "1", "-1"];

    let args: Vec<String> = env::args().collect();
    let file = &args[1];
    let content = fs::read_to_string(file).expect("Something went wrong reading the file");

    println!("{:#?}", content);
    let ast = syn::parse_file(&content);
    // for item in ast.items.iter() {
    //     match item {
    //         _ => {
    //             print_type_of(item)
    //         },
    //     }
    // }
    // println!("Hello");

    let lines = content.split("\r\n");
    for line in lines {
        // println!("{:#?}", line);
        let mut expr = syn::parse_str::<Stmt>(line);
        match expr {
            // statements are divided into 4 types(https://docs.rs/syn/1.0.30/syn/enum.Stmt.html)
            Ok(stmt) => {
                match stmt {
                    syn::Stmt::Local(local) => { // local let binding
                        // println!(" > {:#?}", &local);
                        // utils::print_type_of(&local.init);
                    }
                    syn::Stmt::Item(item) => {
                        // constant statement, use statement, ...(listed here : https://docs.rs/syn/1.0.30/syn/enum.Item.html)
                        match item {
                            syn::Item::Const(itemConst) => {
                                // println!("{}", line);
                                // println!("{:#?}", &itemConst);
                                let mut const_expr: Vec<_> = line.split("=").collect();
                                constants.push(const_expr[1].trim_end_matches(";").trim());
                            }
                            _ => {}
                        }
                    }
                    syn::Stmt::Expr(expr) => {
                        // println!("{:#?}", expr);
                    }
                    syn::Stmt::Semi(expr, semi) => {
                        println!("not a case");
                    }
                }
            }
            Err(error) => {
                // syntax error of target file
                println!("{}", error);
            }
        }
    }

    let mut lines_vec: Vec<_> = content.split("\r\n").collect();
    let (start, end) = find_min_parsable_lines(lines_vec.clone(), num_line);
    let line_to_parse = lines_vec[start..end].join("\t\n");
    let expr_to_mutate = syn::parse_str::<Stmt>(&line_to_parse);
    println!("{:?}", expr_to_mutate);
    match expr_to_mutate {
        Ok(stmt) => {
            println!("{:#?}", stmt);
            match stmt {
                syn::Stmt::Local(local) => {
                    // let binding
                }
                syn::Stmt::Item(item) => {
                    // constant statement, use statement, ...(listed here : https://docs.rs/syn/1.0.30/syn/enum.Item.html)
                    match item {
                        syn::Item::Const(itemConst) => {
                            let mut new_constant_vec: Vec<_> = constants
                                .choose_multiple(&mut rand::thread_rng(), 1)
                                .collect();
                            let mut new_constant = new_constant_vec[0];
                            let mut const_expr: Vec<_> = line_to_parse.split("=").collect();
                            while const_expr[1].trim_end_matches(";").trim() == *new_constant {
                                new_constant_vec = constants
                                    .choose_multiple(&mut rand::thread_rng(), 1)
                                    .collect();
                                new_constant = new_constant_vec[0];
                            }
                            let tmp = const_expr[0].to_string();
                            let const_string =
                                tmp + &("= ".to_string()) + new_constant + &(";".to_string());
                            lines_vec[num_line - 1] = &const_string;
                            return lines_vec.join("\t\r");
                        }
                        _ => { () },
                    }
                }
                syn::Stmt::Expr(expr) => { () },
                syn::Stmt::Semi(expr, semi) => { () },
                _ => { () },
            }
        }
        Err(error) => {
            // println!("{}", error);
        }
    }
    return "hello".to_string(); // temporary return value
}

pub fn mutate_file_by_line3(file: String, num_line: usize) -> String {
    let args: Vec<String> = env::args().collect();
    let example_source = fs::read_to_string(&file).expect("Something went wrong reading the file");
    
    let mut _binopvisitor = BinOpVisitor { BinOps: Vec::new(), Line: num_line, Column: 0, Covered: HashSet::new(), Prevsize: 0};

    // If rustfmt doesn't exist, install it
    Command::new("rustup")
            .arg("component")
            .arg("add")
            .arg("rustfmt")
            .spawn()
            .expect("rustup command failed to start");
    
    loop {
        // Since there can be multiple same operators in same line,
        // so read original source code everytime and mutate only one operator at once
        let mut syntax_tree = syn::parse_file(&example_source).unwrap();
        _binopvisitor.visit_file_mut(&mut syntax_tree);

        if _binopvisitor.Prevsize == _binopvisitor.Covered.len() {
            break;
        } else {
            let mut fz = fs::File::create(format!("{}{}{}", "mutated",_binopvisitor.Column,".rs")).unwrap();
    
            fz.write_all(quote!(#syntax_tree).to_string().as_bytes());
            _binopvisitor.Prevsize += 1;
    
            // Format mutated source code.
            Command::new("rustfmt")
                    .arg(format!("{}{}{}", "mutated",_binopvisitor.Column,".rs"))
                    .spawn()
                    .expect("rustfmt command failed to start");
        }
    }

    return "hello".to_string(); // temporary return value
}