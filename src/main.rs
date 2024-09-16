#![allow(dead_code)]
use std::collections::BTreeMap;

fn main() {
    println!("Hello, world!");
}

const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyzæøå ";

struct Bits {
    bits: Vec<u8>,
    size: u32,
}

struct LzwEncoder {
    dict: BTreeMap<Vec<u8>, Bits>,
    word_size: u32,
}

struct LzwDecoder {
    bitstream: Vec<u8>,
    dict: BTreeMap<Bits, Vec<u8>>,
    word_size: u32,
}

impl LzwEncoder {
    fn new() -> Self {
        let mut dict = BTreeMap::<Vec<u8>, Bits>::new();

        let word_size = (dict.len() - 1).ilog2() + 1;

        LzwEncoder { dict, word_size }
    }
}

impl LzwDecoder {}

fn concat_bits(bits1: Vec<u8>, bits2: Vec<u8>, unused_bits: u8, word_size: u32) -> Vec<u8> {
    todo!()
}
