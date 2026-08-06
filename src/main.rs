mod plugin_list;
mod interfaces;
mod matcher;
mod trie;
mod content;
mod scanner;
mod filter;
mod identifier_set;
#[cfg(test)]
mod tests;


pub use content::*;
pub use matcher::*;
pub use interfaces::*;
pub use filter::*;
pub use identifier_set::*;

fn main() {
    println!("Hello, world!");
}

/*
1. Fac un Content (nu am citit nimic inca)
2. aplic - filtru si ies
3. daca trece de filtru -  extensia / numele --> obtin tipul + content


*/
