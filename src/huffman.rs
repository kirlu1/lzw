use std::collections:: BTreeMap;

use crate::{usize_to_u8, Bits};

pub struct HuffmanEncoder {

}

pub struct HuffmanDecoder {
    
}

pub struct HuffmanLeaf {
    codeword : Bits,
    byte_value : u8,
    subtree_id : u8,
    subtree_weight : u32,
}

impl HuffmanEncoder {
    pub fn new() -> Self {
        HuffmanEncoder {}
    }

    pub fn encode(self, input : &[u8]) -> Vec<u8> {
        let mut counts = [0;256];

        for b in input {
            counts[*b as usize] += 1;
        }

        let transmitted_counts = counts.clone().map(|n| n as u16);
        

        let codeword_tree = construct_tree(&counts);

        let tree_map = codeword_tree
            .into_iter()
            .map(|leaf| (leaf.byte_value, leaf.codeword))
            .collect::<BTreeMap<_,_>>();

        let mut output = Bits::new(vec![], 0);

        for b in input.into_iter() {
            let codeword = tree_map.get(b).expect("Byte value must exist in encoding tree").clone();
            output = output.concat(codeword);
        }


        // First 512 bytes are dedicated to character frequency
        let mut transmission = Vec::from(bytemuck::cast_slice::<u16, u8>(&transmitted_counts));

        let output = usize_to_u8(&output.bits);

        transmission.extend(output);

        transmission
    }
}

impl HuffmanDecoder {
    pub fn new() -> Self {
        HuffmanDecoder {}
    }

    pub fn decode(self, input : &[u8]) -> Vec<u8> {
        let counts : &[u16] = bytemuck::cast_slice::<u8, u16>(&input[0..512]);
        let input = &input[512..];

        let codeword_tree = construct_tree(counts);

        let min_word = codeword_tree.iter().map(|leaf| leaf.codeword.size).min().expect("tree must have leaves");
        let max_word = codeword_tree.iter().map(|leaf| leaf.codeword.size).max().expect("tree must have leaves");

        let tree_map = codeword_tree
            .into_iter()
            .map(|leaf| {
                let size = leaf.codeword.size;
                let useful = size.div_ceil(8);
                let bits = usize_to_u8(&leaf.codeword.shift_left().bits[..])
                    .into_iter()
                    .take(useful)
                    .collect::<Vec<u8>>()
                ;
                ((bits,size), leaf.byte_value)
            })
            .collect::<BTreeMap<_,_>>();

        let mut idx = 0usize;
        let mut bit_idx = 0usize;
        let mut word_size = min_word;
        let mut output = vec![];

        while idx < input.len() && word_size <= max_word {
            let start_idx = idx;
            let end_idx = idx + (word_size+bit_idx).div_ceil(8);
            let new_bit_idx = (bit_idx + word_size as usize) % 8;

            if input.len() <= end_idx {
                break
            }
            let mut codeword = input[start_idx..=end_idx-1].to_vec();

            // Codewords in map are left-adjusted
            codeword[0] <<= bit_idx;
            if 0 < bit_idx {
                for i in 0..codeword.len()-1 {
                    codeword[i] |= codeword[i+1] >> (8 - bit_idx);
                    codeword[i+1] <<= bit_idx;
                }
            }
            while codeword.len() > word_size.div_ceil(8) {
                codeword.pop();
            }

            // Clear irrelevant bits
            let right_space = codeword.len() * 8 - word_size;
            let last = codeword.last_mut().unwrap();
            *last &= u8::MAX << right_space;


            match tree_map.get(&(codeword, word_size)) {
                Some(byte) => {
                    word_size = min_word;
                    output.push(*byte);
                },
                None => {
                    word_size+=1;
                    continue
                }
            }
            
            bit_idx = new_bit_idx;
            idx = end_idx-1;
            if bit_idx == 0 {
                idx+=1;
            }
        }

        output
    }
}


fn construct_tree(character_frequency : &[u16]) -> Vec<HuffmanLeaf> {
    let total = character_frequency.iter().sum();


    let mut counts = character_frequency
        .iter()
        .enumerate()
        .filter(|(_,n)| 0 < **n)
        .map(|(a,b)| (a,*b))
        .collect::<Vec<(usize,u16)>>();

    counts.sort_by_key(|(_,n)| *n);

    let mut tree = counts
        .into_iter()
        .map(|(b, n)| HuffmanLeaf {
            codeword : Bits::new(vec![], 0),
            byte_value : b as u8,
            subtree_id : b as u8,
            subtree_weight : n as u32,
        })
        .collect::<Vec<HuffmanLeaf>>();

    while (tree[0].subtree_weight as u16) != total {
        let mut min_id_weight = (tree[0].subtree_id, tree[0].subtree_weight);
        let mut min2_id_weight = (tree[0].subtree_id, u32::MAX);

        // Identify two subtrees of least weight
        for leaf in tree.iter() {
            if leaf.subtree_id == min_id_weight.0 || leaf.subtree_id == min2_id_weight.0 {
                continue
            }
            if leaf.subtree_weight < min_id_weight.1 {
                min2_id_weight = min_id_weight;
                min_id_weight = (leaf.subtree_id, leaf.subtree_weight);
            } else if leaf.subtree_weight < min2_id_weight.1 {
                min2_id_weight = (leaf.subtree_id, leaf.subtree_weight);
            }
        }

        // Combine leaves into new subtree
        let new_id = min_id_weight.0;
        let combined_weight = min_id_weight.1 + min2_id_weight.1;
        if combined_weight == 0 {
            break
        }

        let new_subtree_leaves = tree
            .iter_mut()
            .filter(|leaf| leaf.subtree_id == min2_id_weight.0 || leaf.subtree_id == min_id_weight.0 );

        // One side of new subtree is 0, one side is 1
        for leaf in new_subtree_leaves {
            if leaf.subtree_id == min_id_weight.0 {
                leaf.codeword.push_from_left(0);
            } else {
                leaf.codeword.push_from_left(1);
            }

            leaf.subtree_id = new_id;
            leaf.subtree_weight = combined_weight;
        }
    }

    tree
}
