use crate::errors::MultiSeqAlignError;

#[inline]
pub fn first_sequence_length(sequences: &[String]) -> usize {
    match sequences.get(0) {
        Some(seq) => seq.len(),
        None => 0_usize,
    }
}

// Returns a tuple of vector of indices and vector of found lengths
#[inline]
pub fn check_unequal_lengths(
    seqs: &[String],
    names: &[String],
    expected: usize,
) -> Result<(), MultiSeqAlignError> {
    let mismatches: (Vec<usize>, Vec<usize>) = seqs
        .iter()
        .enumerate()
        .filter_map(|(index, collection)| {
            if collection.len() == expected {
                None
            } else {
                Some((index, collection.len()))
            }
        })
        .unzip();

    if mismatches.0.is_empty() {
        Ok(())
    } else {
        Err(MultiSeqAlignError::MultipleSequencesOfDifferentLengths {
            expected_length: expected,
            sequences_names: mismatches
                .0
                .iter()
                .map(|i| names.get(*i).unwrap().clone())
                .collect(),
            found_lengths: mismatches.1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn unequal_lengths_1() {
        let error = crate::utils::check_unequal_lengths(
            &[String::from("ILK"), String::from("ILKS")],
            &[String::from("seq1"), String::from("seq2")],
            3,
        )
        .err()
        .unwrap();
        let expected = MultiSeqAlignError::MultipleSequencesOfDifferentLengths {
            expected_length: 3,
            sequences_names: vec![String::from("seq2")],
            found_lengths: vec![4],
        };
        assert_eq!(error, expected);
    }

    #[test]
    fn unequal_lengths_2() {
        let error = crate::utils::check_unequal_lengths(
            &[
                String::from("ILK"),
                String::from("ILKS"),
                String::from("ILK"),
                String::from("SILKS"),
            ],
            &[
                String::from("seq1"),
                String::from("seq2"),
                String::from("seq3"),
                String::from("seq4"),
            ],
            3,
        )
        .err()
        .unwrap();
        let expected = MultiSeqAlignError::MultipleSequencesOfDifferentLengths {
            expected_length: 3,
            sequences_names: vec![String::from("seq2"), String::from("seq4")],
            found_lengths: vec![4, 5],
        };
        assert_eq!(error, expected);
    }
}