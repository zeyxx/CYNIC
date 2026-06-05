//! Tokenizer primitives: BPE, Unigram, and SentencePiece-style tokenization.
//! Project 1: Build a tokenizer from scratch.
//!
//! Reference: Sennrich et al. 2016 (BPE), Kudo 2018 (SentencePiece/Unigram)

use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};

pub const PAD_ID: u32 = 0;
pub const UNK_ID: u32 = 1;
pub const BOS_ID: u32 = 2;
pub const EOS_ID: u32 = 3;
pub const PAD_TOKEN: &str = "<pad>";
pub const UNK_TOKEN: &str = "<unk>";
pub const BOS_TOKEN: &str = "<s>";
pub const EOS_TOKEN: &str = "</s>";
pub const DEFAULT_VOCAB_SIZE: usize = 32000;
pub const MIN_MERGE_FREQ: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenizerAlgorithm { BPE, Unigram }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub id: u32,
    pub text: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vocabulary {
    tokens: Vec<Token>,
    token_to_id: HashMap<String, u32>,
    merges: Vec<(String, String)>,
    scores: HashMap<u32, f64>,
}

impl Vocabulary {
    pub fn new() -> Self {
        let mut vocab = Self {
            tokens: Vec::new(),
            token_to_id: HashMap::new(),
            merges: Vec::new(),
            scores: HashMap::new(),
        };
        vocab.add_special_tokens();
        vocab
    }

    fn add_special_tokens(&mut self) {
        for (id, text) in [(PAD_ID, PAD_TOKEN), (UNK_ID, UNK_TOKEN), (BOS_ID, BOS_TOKEN), (EOS_ID, EOS_TOKEN)] {
            let token = Token { id, text: text.to_string(), score: 0.0 };
            self.tokens.push(token);
            self.token_to_id.insert(text.to_string(), id);
            self.scores.insert(id, 0.0);
        }
    }

    pub fn len(&self) -> usize { self.tokens.len() }
    pub fn get_id(&self, text: &str) -> Option<u32> { self.token_to_id.get(text).copied() }
    pub fn get_text(&self, id: u32) -> Option<&str> { self.tokens.iter().find(|t| t.id == id).map(|t| t.text.as_str()) }
    pub fn add_token(&mut self, text: String, score: f64) -> u32 {
        let id = self.tokens.len() as u32;
        self.scores.insert(id, score);
        self.tokens.push(Token { id, text: text.clone(), score });
        self.token_to_id.insert(text, id);
        id
    }
    pub fn add_merge(&mut self, left: String, right: String) { self.merges.push((left, right)); }
    pub fn merges(&self) -> &[(String, String)] { &self.merges }
    pub fn score(&self, id: u32) -> f64 { *self.scores.get(&id).unwrap_or(&f64::NEG_INFINITY) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tokenizer {
    pub algorithm: TokenizerAlgorithm,
    pub vocabulary: Vocabulary,
    pub vocab_size: usize,
    pub byte_fallback: bool,
}

impl Tokenizer {
    pub fn new(algorithm: TokenizerAlgorithm, vocab_size: usize) -> Self {
        Self { algorithm, vocabulary: Vocabulary::new(), vocab_size, byte_fallback: true }
    }

    pub fn encode(&self, text: &str) -> Result<Vec<u32>, TokenizerError> {
        match self.algorithm {
            TokenizerAlgorithm::BPE => self.encode_bpe(text),
            TokenizerAlgorithm::Unigram => self.encode_unigram(text),
        }
    }

    pub fn decode(&self, ids: &[u32]) -> Result<String, TokenizerError> {
        let mut result = String::new();
        for &id in ids {
            if let Some(text) = self.vocabulary.get_text(id) {
                if id >= 4 { result.push_str(text); }
            } else if self.byte_fallback && id < 256 {
                result.push(id as u8 as char);
            }
        }
        Ok(result)
    }

    fn encode_bpe(&self, text: &str) -> Result<Vec<u32>, TokenizerError> {
        let mut ids = Vec::new();
        for word in text.split_whitespace() {
            let mut tokens: Vec<u32> = word.bytes()
                .map(|b| self.vocabulary.get_id(&format!("{:02X}", b)).unwrap_or(UNK_ID))
                .collect();
            for (left, right) in &self.vocabulary.merges {
                if let (Some(lid), Some(rid), Some(mid)) = (
                    self.vocabulary.get_id(left),
                    self.vocabulary.get_id(right),
                    self.vocabulary.get_id(&format!("{}{}", left.trim_start_matches("##"), right.trim_start_matches("##"))),
                ) {
                    let mut new = Vec::new();
                    let mut i = 0;
                    while i < tokens.len() {
                        if i + 1 < tokens.len() && tokens[i] == lid && tokens[i + 1] == rid {
                            new.push(mid); i += 2;
                        } else { new.push(tokens[i]); i += 1; }
                    }
                    tokens = new;
                }
            }
            ids.extend(tokens);
        }
        Ok(ids)
    }

    fn encode_unigram(&self, text: &str) -> Result<Vec<u32>, TokenizerError> {
        let mut ids = Vec::new();
        for word in text.split_whitespace() {
            let chars: Vec<char> = word.chars().collect();
            let n = chars.len();
            let mut best = vec![f64::NEG_INFINITY; n + 1];
            let mut back: Vec<(usize, u32)> = vec![(0, UNK_ID); n + 1];
            best[0] = 0.0;
            for i in 0..n {
                if best[i] == f64::NEG_INFINITY { continue; }
                for j in i + 1..=n.min(i + 20) {
                    let sub: String = chars[i..j].iter().collect();
                    if let Some(&tid) = self.vocabulary.token_to_id.get(&sub) {
                        let s = best[i] + self.vocabulary.score(tid);
                        if s > best[j] { best[j] = s; back[j] = (i, tid); }
                    }
                }
                if best[i + 1] == f64::NEG_INFINITY && self.byte_fallback {
                    let byte = format!("{:02X}", chars[i] as u32);
                    if let Some(&bid) = self.vocabulary.token_to_id.get(&byte) {
                        let s = best[i] + self.vocabulary.score(bid);
                        if s > best[i + 1] { best[i + 1] = s; back[i + 1] = (i, bid); }
                    }
                }
            }
            let mut result = Vec::new();
            let mut pos = n;
            while pos > 0 { let (p, t) = back[pos]; result.push(t); pos = p; }
            result.reverse();
            ids.extend(result);
        }
        Ok(ids)
    }
}

// ── Trainers ─────────────────────────────────────────────────

#[derive(Debug)]
pub struct BPETrainer { vocab_size: usize, min_frequency: usize }

impl BPETrainer {
    pub fn new(vocab_size: usize) -> Self { Self { vocab_size, min_frequency: MIN_MERGE_FREQ } }
    pub fn with_min_frequency(mut self, f: usize) -> Self { self.min_frequency = f; self }

    pub fn train(&self, corpus: &[String]) -> Result<Tokenizer, TokenizerError> {
        let mut tok = Tokenizer::new(TokenizerAlgorithm::BPE, self.vocab_size);
        tok.vocabulary = Vocabulary::new();
        let mut char_freq: HashMap<char, usize> = HashMap::new();
        for text in corpus { for ch in text.chars() { *char_freq.entry(ch).or_insert(0) += 1; } }
        for byte in 0u8..=255u8 { tok.vocabulary.add_token(format!("{:02X}", byte), 0.0); }
        let mut sorted: Vec<_> = char_freq.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        for (ch, freq) in sorted {
            if freq >= self.min_frequency && tok.vocabulary.len() < self.vocab_size {
                tok.vocabulary.add_token(format!("{:04X}", ch as u32), freq as f64);
            }
        }
        let mut wf: HashMap<Vec<u32>, usize> = HashMap::new();
        for text in corpus {
            for word in text.split_whitespace() {
                let ids: Vec<u32> = word.bytes().map(|b| tok.vocabulary.get_id(&format!("{:02X}", b)).unwrap_or(UNK_ID)).collect();
                if !ids.is_empty() { *wf.entry(ids).or_insert(0) += 1; }
            }
        }
        let mut added = 0;
        while tok.vocabulary.len() < self.vocab_size && added < self.vocab_size - 256 {
            let mut pf: HashMap<(u32, u32), usize> = HashMap::new();
            for (w, f) in &wf {
                for i in 0..w.len().saturating_sub(1) { *pf.entry((w[i], w[i + 1])).or_insert(0) += f; }
            }
            let best = pf.into_iter().filter(|(_, f)| *f >= self.min_frequency).max_by_key(|(_, f)| *f);
            if let Some(((l, r), freq)) = best {
                let lt = tok.vocabulary.get_text(l).unwrap_or("").to_string();
                let rt = tok.vocabulary.get_text(r).unwrap_or("").to_string();
                let mt = format!("{}{}", lt, rt);
                let mid = tok.vocabulary.add_token(mt.clone(), freq as f64);
                tok.vocabulary.add_merge(lt, rt);
                let mut nw = HashMap::new();
                for (w, f) in wf {
                    let mut v = Vec::new(); let mut i = 0;
                    while i < w.len() {
                        if i + 1 < w.len() && w[i] == l && w[i + 1] == r { v.push(mid); i += 2; }
                        else { v.push(w[i]); i += 1; }
                    }
                    *nw.entry(v).or_insert(0) += f;
                }
                wf = nw; added += 1;
            } else { break; }
        }
        Ok(tok)
    }
}

#[derive(Debug)]
pub struct UnigramTrainer { vocab_size: usize, num_iterations: usize }

impl UnigramTrainer {
    pub fn new(vocab_size: usize) -> Self { Self { vocab_size, num_iterations: 10 } }
    pub fn with_iterations(mut self, i: usize) -> Self { self.num_iterations = i; self }

    pub fn train(&self, corpus: &[String]) -> Result<Tokenizer, TokenizerError> {
        let mut tok = Tokenizer::new(TokenizerAlgorithm::Unigram, self.vocab_size);
        tok.vocabulary = Vocabulary::new();
        let mut sf: HashMap<String, usize> = HashMap::new();
        for text in corpus {
            let chars: Vec<char> = text.chars().collect();
            for i in 0..chars.len() {
                for len in 1..=10.min(chars.len() - i) {
                    let sub: String = chars[i..i + len].iter().collect();
                    *sf.entry(sub).or_insert(0) += 1;
                }
            }
        }
        let mut sorted: Vec<_> = sf.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(self.vocab_size);
        for (sub, freq) in sorted { tok.vocabulary.add_token(sub, (freq as f64).ln()); }
        for _ in 0..self.num_iterations { self.em_step(&mut tok, corpus)?; }
        Ok(tok)
    }

    fn em_step(&self, tok: &mut Tokenizer, corpus: &[String]) -> Result<(), TokenizerError> {
        let mut tc: HashMap<u32, usize> = HashMap::new();
        let mut total = 0usize;
        for text in corpus {
            for word in text.split_whitespace() {
                for id in tok.encode_unigram(word)? {
                    if id >= 4 { *tc.entry(id).or_insert(0) += 1; total += 1; }
                }
            }
        }
        for t in &mut tok.vocabulary.tokens {
            if t.id >= 4 { t.score = (tc.get(&t.id).copied().unwrap_or(1) as f64 / total.max(1) as f64).ln(); }
        }
        let mut st: Vec<_> = tok.vocabulary.tokens[4..].iter().map(|t| (t.id, t.score)).collect();
        st.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        st.truncate(self.vocab_size.saturating_sub(4));
        let keep: HashSet<u32> = [PAD_ID, UNK_ID, BOS_ID, EOS_ID].into_iter().chain(st.iter().map(|(id, _)| *id)).collect();
        tok.vocabulary.tokens.retain(|t| keep.contains(&t.id));
        tok.vocabulary.token_to_id.retain(|_, id| keep.contains(id));
        tok.vocabulary.scores.retain(|id, _| keep.contains(id));
        for (nid, t) in tok.vocabulary.tokens.iter_mut().enumerate() { t.id = nid as u32; }
        tok.vocabulary.token_to_id.clear();
        for t in &tok.vocabulary.tokens { tok.vocabulary.token_to_id.insert(t.text.clone(), t.id); }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TokenizerError {
    #[error("Invalid UTF-8")] InvalidUtf8,
    #[error("Token not found: {0}")] TokenNotFound(String),
    #[error("Not trained")] NotTrained,
    #[error("IO: {0}")] Io(#[from] std::io::Error),
    #[error("Serde: {0}")] Serde(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerStats {
    pub total_tokens: usize,
    pub total_chars: usize,
    pub compression_ratio: f64,
}

impl TokenizerStats {
    pub fn new() -> Self { Self { total_tokens: 0, total_chars: 0, compression_ratio: 0.0 } }
    pub fn add(&mut self, text: &str, ids: &[u32]) {
        self.total_chars += text.chars().count();
        self.total_tokens += ids.len();
        self.compression_ratio = self.total_chars as f64 / self.total_tokens.max(1) as f64;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerComparison {
    pub tokenizer_a_name: String,
    pub tokenizer_b_name: String,
    pub stats_a: TokenizerStats,
    pub stats_b: TokenizerStats,
}

pub fn compare_tokenizers(a: &Tokenizer, b: &Tokenizer, corpus: &[String]) -> TokenizerComparison {
    let (mut sa, mut sb) = (TokenizerStats::new(), TokenizerStats::new());
    for text in corpus {
        if let Ok(ids) = a.encode(text) { sa.add(text, &ids); }
        if let Ok(ids) = b.encode(text) { sb.add(text, &ids); }
    }
    TokenizerComparison { tokenizer_a_name: format!("{:?}", a.algorithm), tokenizer_b_name: format!("{:?}", b.algorithm), stats_a: sa, stats_b: sb }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpe_train_and_encode() {
        let tok = BPETrainer::new(100).train(&vec!["hello world".into(), "hello there".into()]).unwrap();
        assert!(tok.vocabulary.len() > 4);
        let ids = tok.encode("hello").unwrap();
        assert!(!ids.is_empty());
    }

    #[test]
    fn unigram_train_and_encode() {
        let tok = UnigramTrainer::new(100).with_iterations(3).train(&vec!["hello world".into()]).unwrap();
        let ids = tok.encode("hello").unwrap();
        assert!(!ids.is_empty());
    }

    #[test]
    fn special_tokens() {
        let tok = BPETrainer::new(50).train(&vec!["test".into()]).unwrap();
        assert_eq!(tok.vocabulary.get_id(PAD_TOKEN), Some(PAD_ID));
        assert_eq!(tok.vocabulary.get_id(UNK_TOKEN), Some(UNK_ID));
    }

    #[test]
    fn serialization() {
        let tok = BPETrainer::new(50).train(&vec!["hello".into()]).unwrap();
        let json = serde_json::to_string(&tok).unwrap();
        let de: Tokenizer = serde_json::from_str(&json).unwrap();
        assert_eq!(tok.vocabulary.len(), de.vocabulary.len());
    }

    #[test]
    fn compare_tokenizers() {
        let a = BPETrainer::new(50).train(&vec!["hello world".into()]).unwrap();
        let comp = compare_tokenizers(&a, &a, &vec!["hello".into()]);
        assert!(comp.stats_a.compression_ratio > 1.0);
    }
}