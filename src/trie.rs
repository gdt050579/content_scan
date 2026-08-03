type TrieIndex = u16;

#[derive(Debug, Clone, Copy)]
struct TrieStep {
    index: TrieIndex,
    symbol: u8,
}

#[derive(Debug, Clone)]
enum TrieChildren {
    None,
    One(TrieStep),
    Two((TrieStep, TrieStep)),
    Three((TrieStep, TrieStep, TrieStep)),
    Many(Vec<TrieStep>),
}
impl TrieChildren {
    pub fn find(&self, symbol: u8) -> Option<u16> {
        match self {
            TrieChildren::None => None,
            TrieChildren::One(trie_step) => {
                if trie_step.symbol == symbol {
                    Some(trie_step.index)
                } else {
                    None
                }
            }
            TrieChildren::Two((ts1, ts2)) => {
                if ts1.symbol == symbol {
                    Some(ts1.index)
                } else if ts2.symbol == symbol {
                    Some(ts2.index)
                } else {
                    None
                }
            }
            TrieChildren::Three((ts1, ts2, ts3)) => {
                if ts1.symbol == symbol {
                    Some(ts1.index)
                } else if ts2.symbol == symbol {
                    Some(ts2.index)
                } else if ts3.symbol == symbol {
                    Some(ts3.index)
                } else {
                    None
                }
            }
            TrieChildren::Many(v) => {
                // binary search - v is sorted based on the symbol
                v.binary_search_by_key(&symbol, |trie_step| trie_step.symbol)
                    .ok()
                    .map(|index| v[index].index)
            }
        }
    }
    fn insert(&mut self, symbol: u8, index: u16) {
        match self {
            TrieChildren::None => {
                *self = TrieChildren::One(TrieStep { index, symbol });
            }
            TrieChildren::One(trie_step) => {
                if trie_step.symbol != symbol {
                    *self = TrieChildren::Two((*trie_step, TrieStep { index, symbol }));
                }
            }
            TrieChildren::Two((ts1, ts2)) => {
                if ts1.symbol != symbol && ts2.symbol != symbol {
                    *self = TrieChildren::Three((*ts1, *ts2, TrieStep { index, symbol }));
                }
            }
            TrieChildren::Three((ts1, ts2, ts3)) => {
                if ts1.symbol != symbol && ts2.symbol != symbol && ts3.symbol != symbol {
                    let mut v = vec![*ts1, *ts2, *ts3, TrieStep { index, symbol }];
                    v.sort_by_key(|ts| ts.symbol);
                    *self = TrieChildren::Many(v);
                }
            }
            TrieChildren::Many(v) => {
                // binary search - v is sorted based on the symbol
                if let Err(vec_index) = v.binary_search_by_key(&symbol, |ts| ts.symbol) {
                    v.insert(vec_index, TrieStep { index, symbol });
                }
            }
        }
    }
}
struct TrieNode {
    children: TrieChildren,
    value: Option<u16>,
}
pub struct Trie {
    nodes: Vec<TrieNode>,
}
impl Trie {
    pub fn scan(&self, data: &[u8]) -> Option<u16> {
        let mut current_index: usize = 0;
        let mut current_value = self.nodes[0].value;
        for symbol in data {
            match self.nodes[current_index].children.find(*symbol) {
                Some(next) => {
                    current_index = next as usize;
                    if let Some(v) = self.nodes[current_index].value {
                        current_value = Some(v);
                    }
                }
                None => break,
            }
        }
        current_value
    }
}

struct Word {
    data: &'static [u8],
    value: u16,
}
pub struct TrieBuilder {
    words: Vec<Word>,
}
impl TrieBuilder {
    pub fn new() -> Self {
        Self { words: Vec::new() }
    }
    pub fn add(&mut self, data: &'static [u8], value: u16) {
        self.words.push(Word { data, value });
    }
    pub fn build(self) -> Trie {
        let mut nodes: Vec<TrieNode> = Vec::new();
        nodes.push(TrieNode {
            children: TrieChildren::None,
            value: None,
        });

        for word in &self.words {
            let mut current: usize = 0;
            for &symbol in word.data {
                match nodes[current].children.find(symbol) {
                    Some(next) => current = next as usize,
                    None => {
                        let new_index = nodes.len();
                        assert!(
                            new_index <= TrieIndex::MAX as usize,
                            "trie exceeds u16 node capacity"
                        );
                        nodes.push(TrieNode {
                            children: TrieChildren::None,
                            value: None,
                        });
                        nodes[current]
                            .children
                            .insert(symbol, new_index as TrieIndex);
                        current = new_index;
                    }
                }
            }
            nodes[current].value = Some(word.value);
        }

        Trie { nodes }
    }
}
