mod analyzer_list;
mod interfaces;
mod matcher;
mod trie;
mod content;
mod scanner;
#[cfg(test)]
mod tests;


pub use content::*;
pub use matcher::*;
pub use interfaces::*;

fn main() {
    println!("Hello, world!");
}

/*
1. Fac un Content (nu am citit nimic inca)
2. aplic - filtru si ies
3. daca trece de filtru -  extensia / numele --> obtin tipul + content


*/
