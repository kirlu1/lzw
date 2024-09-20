use std::borrow::BorrowMut;
use std::char;
use std::collections::BTreeMap;

use std::fs::File;
use std::io::{Read, Write};

type IntType = usize;

const INTTYPE_BITS : IntType = (0 as IntType).count_zeros() as IntType;

const ALPHABET: &str = " abcdefghijklmnopqrstuvwxyzæøå";


fn main() {

    let encoder = LzwEncoder::new();

    let mut buffer = String::new();
    File::open("folktale.txt")
        .expect("failed opening file")
        .read_to_string(&mut buffer)
        .expect("failed reading into buffer");

    buffer = buffer.to_lowercase();
    let input = buffer.chars()
    .collect::<Vec<char>>();

    let uncompressed_size = buffer.len();
    
    let encoded = encoder.encode(input);
    let compressed_size = encoded.bits.len();

    let to_decompress = encoded.bits.clone();
    

    encoded.to_file("compressed.txt");


    println!("Uncompressed size: {}", uncompressed_size);
    println!("Compressed size: {}", compressed_size);

    println!("Percent size of original: {}%", (compressed_size as f64 / uncompressed_size as f64) * 100f64);

    let decoder = LzwDecoder::new();

    let mut uncompressed = decoder.decode(to_decompress);

    File::create("uncompressed.txt").unwrap().write_all(uncompressed.as_bytes()).expect("coulnd't write uncompressed");

}


#[derive(Debug, Clone)]
struct Bits {
    bits: Vec<IntType>,
    size: IntType,
}

struct LzwEncoder {
    dict: BTreeMap<Vec<char>, Vec<IntType>>,
    word_size: IntType,
    next_word: Vec<IntType>,
}

struct LzwDecoder {
    dict: BTreeMap<Vec<IntType>, Vec<char>>,
    word_size: IntType,
    next_word: Vec<IntType>,
}


impl Bits {
    fn new(val : Vec<IntType>, word_size : IntType) -> Self {
        let mut bits = val;

        let available_bits = bits.len() as IntType * INTTYPE_BITS;

        if word_size > available_bits {
            let pad_size = (word_size - available_bits) / INTTYPE_BITS;
            let mut padded = vec![0; pad_size as usize];

            padded.append(&mut bits);
            bits = padded;
        }

        Bits {
            bits,
            size : word_size,
        }
    }

    fn unused(&self) -> IntType {
        self.bits.len() as IntType * INTTYPE_BITS - self.size
    }

    /// Concatenates two Bits into one Bits, with their combined size. 
    /// 
    /// Assumes self is left-adjusted, and other is right-adjusted.
    fn concat(mut self, mut other : Bits) -> Bits {
        let combined_size = self.size + other.size;

        let remainder_shift = self.unused(); // Space left for other's bits
        let filler_shift = INTTYPE_BITS - remainder_shift;

        other = other.shift_left();

        if self.unused() == 0 {
            self.bits.append(&mut other.bits);
            return Bits { bits : self.bits, size : combined_size }
        }

        // Shuffle other's bits backwards
        for i in other.bits {
            let end = self.bits.last_mut().unwrap();
            let fill = i >> filler_shift;
            let remainder = i << remainder_shift;

            *end |= fill;
            self.bits.push(remainder);
        }

        let mut new = Bits { bits : self.bits, size : combined_size };

        while new.unused() >= INTTYPE_BITS {
            new.bits.pop();
        }
        new
    }

    /// Shifts from the right to the left
    fn shift_left(mut self) -> Bits {
        let difference = self.unused();
    
        let mut new_bits = vec![];
    
        self.bits.push(0);
        for w in self.bits.windows(2) {
            let mut a = w[0];
            let mut b = w[1];
    
            a <<= difference;
            b >>= INTTYPE_BITS - difference;
            new_bits.push(a | b);
        }
    
        Bits {
            bits : new_bits,
            size : self.size
        }
    }

    /// Creates a file or truncates an existing one of name `filename`,
    /// then writes the Bits object *as bits* to the file
    fn to_file(self, filename : &str) {
        let mut newfile = File::create(filename).expect("couldn't create file");

        let bytes = bytemuck::cast_slice::<IntType, u8>(&self.bits[..]);

        newfile.write_all(bytes).expect("failed to write entire buffer");
    }

}

impl LzwEncoder {
    fn new() -> Self {
        let dict = BTreeMap::<Vec<char>, Vec<IntType>>::new();

        let next_word = vec![0];

        let mut encoder = LzwEncoder { dict, word_size : 0, next_word };
        
        for c in ALPHABET.chars() {
            encoder.insert(vec![c]);
        }

        encoder
    }

    /// Attempt to insert a new sequence of characters into the dictionary with a new codeword.
    /// 
    /// Returns false if the sequence already has a codeword.
    fn insert(&mut self, sequence : Vec<char>) -> bool {
        if self.dict.contains_key(&sequence) {
            return false;
        }
        
        let codeword = self.next_word();

        self.dict.insert(sequence, codeword);

        self.word_size = ((self.dict.len()-1).checked_ilog2().unwrap_or(0) + 1) as IntType;

        return true
    }

    // Could be put into trait??
    fn next_word(&mut self) -> Vec<IntType> {
        let old = self.next_word.clone();

        for n in self.next_word.iter_mut().rev() {
            *n+=1;
            if *n != 0 {
                break
            }
        }

        if self.next_word[0] == 0 {
            self.next_word.push(0);
            self.next_word[0]+=1;
        }
        return old
    }

    fn encode(mut self, input : Vec<char>) -> Bits {

        let mut output : Bits = Bits { bits : vec![], size : 0 };
        let mut sequence: Vec<char> = vec![];

        
        for i in 0..input.len() {
            // Insertion into dict changes word_size before appending to output
            let current_word_size = self.word_size;

            sequence.push(input[i]);    

            if self.insert(sequence.clone()) {
                // Insert succeeds
                sequence.pop();

                if sequence.len() == 0 {
                    panic!("Alphabet not comprehensive");
                }
                let codeword = self.dict.get(&sequence).expect("encoding should exist").clone();

                sequence.clear();
                sequence.push(input[i]); 

                

                output = output.concat(Bits::new(codeword, current_word_size));
            }
        }

        if !sequence.is_empty() {
            let codeword = self.dict.get(&sequence).expect("encoding should exist 2").clone();
            output = output.concat(Bits::new(codeword, self.word_size));
        }

        output
    }
}


impl LzwDecoder {
    fn new() -> Self {
        let dict = BTreeMap::<Vec<IntType>, Vec<char>>::new();

        let mut decoder = LzwDecoder {
            dict,
            word_size : 0,
            next_word : vec![0],
        };
        
        for c in ALPHABET.chars() {
            let codeword = decoder.next_word();
            decoder.dict.insert(codeword, vec![c]);
        }

        decoder.word_size = ((decoder.dict.len() - 1).checked_ilog2().unwrap_or(0) + 1) as IntType;

        decoder
    }

    fn next_word(&mut self) -> Vec<IntType> {
        let old = self.next_word.clone();

        for n in self.next_word.iter_mut().rev() {
            *n+=1;
            if *n != 0 {
                break
            }
        }

        if self.next_word[0] == 0 {
            self.next_word.push(0);
            self.next_word[0]+=1;
        }
        return old
    }



    fn insert(&mut self, sequence : Vec<char>) -> bool {

        let next_word = self.next_word();
        self.dict.insert(next_word, sequence);

        self.word_size = ((self.dict.len()).checked_ilog2().unwrap_or(0) + 1) as IntType;

        return true
    }


    fn decode(mut self, mut input : Vec<IntType>) -> String {
        let true_length = input.len();
        let ratio = INTTYPE_BITS / 8;
        let disalignment = (ratio - (input.len() as IntType % ratio)) % ratio;

        input.append(&mut vec![0; disalignment.into()]);

        let input : &[IntType] = bytemuck::cast_slice(&input[..]);
        let mut output = vec![];

        let mut bit_idx = 0;
        let mut idx = 0;

        let mut sequence_buffer : Vec<char> = vec![];

        let mut last_inserted = vec![];


        while idx < true_length {
            let start_idx = idx;
            let end_idx = idx + (self.word_size + bit_idx).div_ceil(INTTYPE_BITS) as usize;
            let new_bit_idx = bit_idx + self.word_size;

            let mut codeword = input[start_idx..=end_idx-1].to_vec();
            let subslice_length = codeword.len();

            // Zero out irrelevant bits
            codeword[0] &= IntType::MAX >> bit_idx;

            // Shift entire sequence to the right, by number of irrelevant bits
            let right_space = codeword.len() as IntType * INTTYPE_BITS - bit_idx - self.word_size;


            codeword[subslice_length-1] >>= right_space;
            for i in (1..subslice_length).rev() {
                codeword[i] |= codeword[i-1] << (INTTYPE_BITS - right_space);
                codeword[i-1] >>= right_space;
            }

            // Remove irrelevant 0 elements to make sure sequence exists in dictionary
            codeword = codeword.into_iter().skip_while(|a| *a==0).collect::<Vec<IntType>>();
            if codeword.len() == 0 {
                codeword = vec![0];
            }

            let characters = self.dict.get(&codeword).unwrap_or_else(
                || {
                    last_inserted.push(last_inserted[0]);
                    &last_inserted
                }
            ).clone();

            output.extend(characters.clone());

            if !sequence_buffer.is_empty() {
                sequence_buffer.push(characters[0]);
                self.insert(sequence_buffer.clone());
                // Handle cScSc case
                last_inserted = sequence_buffer.clone();

                sequence_buffer.clear()
            }

            sequence_buffer.extend(characters);
            
            bit_idx = new_bit_idx % INTTYPE_BITS;
            idx = end_idx-1;
            if bit_idx == 0 {
                idx+=1;
            }
        }


        output
            .into_iter()
            .collect::<String>()
    }
}
