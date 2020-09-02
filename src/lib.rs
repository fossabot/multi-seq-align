/*! ![stability-experimental](https://img.shields.io/badge/stability-experimental-orange.svg)

A crate to manipulate multiple sequences alignments in Rust.

Instead of storing aligned sequences as multiple strings, `multi_seq_align` stores bases or residues in [`Alignment`] using a list of characters, like a matrix. This allows easy access to specific rows and columns of the alignment.

# Usage

```rust
# use multi_seq_align::Alignment;
# use std::error::Error;
# fn main() -> Result<(), Box<dyn Error>> {
let mut kappa_casein_fragments_alignment = Alignment::create(
    vec![
        "P06796".to_string(), // Mouse
        "P07498".to_string(), // Human
        "P02668".to_string(), // Cattle
    ],
    vec![
        "CASK_MOUSE".to_string(),
        "CASK_HUMAN".to_string(),
        "CASK_BOVIN".to_string(),
    ],
    &[
        b"PAPISKWQSMP".to_vec(),
        b"HAQIPQRQYLP".to_vec(),
        b"PAQILQWQVLS".to_vec(),
    ],
)?;

// Let's extract a column of this alignment
assert_eq!(
    kappa_casein_fragments_alignment.nth_position(6).unwrap(),
    [&b'W', &b'R', &b'W']
);

// But we also have the aligned sequence for the Platypus
// Let's add it to the original alignment
kappa_casein_fragments_alignment.add_aligned_sequence(
    "D0QJA9".to_string(),
    "D0QJA9_ORNAN".to_string(),
    b"EHQRP--YVLP".to_vec(),
)?;

// the new aligned sequence has a gap at the 6th position
assert_eq!(
    kappa_casein_fragments_alignment.nth_position(6).unwrap(),
    [&b'W', &b'R', &b'W', &b'-']
);

// We can also loop over each position of the alignment
for aas in kappa_casein_fragments_alignment.iter_positions() {
    println!("{:?}", aas);
    assert_eq!(aas.len(), 4); // 4 sequences
}

# Ok(())
# }
```

Here I instancied an alignment using `u8`, but `Alignment` works on generics like numbers, custom or third-party structs.

# Features

- Create [`Alignment`] from one or multiple aligned sequences at once (see [`add_aligned_sequence()`] and [`create()`]).
- Extract columns of the alignment (see [`nth_position()`]).
This crate is currently in early stage development. I wouldn't recommend using it in production but I am interested in possible ideas to further the developemt of this project. Quite some work needs toi be done to improve the API and make it easy to use in other project.
# Ideas
- Computation of conservation scores
- Identification of conserved sites
- Computation of consensus sequence
- Collapse / trim alignment
- Serialisation / Deserialisation of alignment files
- Extract sub-alignments
    - positions
    - motifs

# Optimisation

My goal is to reduce the footprint of this crate, there is ome work to do to achieve it. The code will eventually be optimised to be faster and to better use memory.

[`Alignment`]: struct.Alignment.html
[`nth_position()`]: struct.Alignment.html#method.nth
[`add_aligned_sequence()`]: struct.Alignment.html#method.add_aligned_sequence
[`create()`]: struct.Alignment.html#method.create
*/
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

mod errors;
mod utils;

use errors::MultiSeqAlignError;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

/// An alignment of DNA or amino acids sequences
///
/// Aligned sequences should all have the same length. Each sequence is stored as one vector of `char`s. This allows an easy access to columns and rows of the alignment.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)] // Use Rc to implement Copy
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Alignment<T> {
    /// Sequence names
    names: Vec<String>,
    /// Sequence descriptions
    descriptions: Vec<String>,
    /// Sequences (as one)
    sequences: Vec<T>,
    /// The number of sequences in the alignment
    n_sequences: usize,
    /// The length of the alignment
    length: usize,
}

impl<T> Default for Alignment<T>
where
    T: Clone,
{
    fn default() -> Self {
        Self {
            names: Vec::<String>::default(),
            descriptions: Vec::<String>::default(),
            sequences: Vec::<T>::default(),
            n_sequences: 0_usize,
            length: 0_usize,
        }
    }
}
struct AlignmentPositionIterator<'a, T> {
    alignment: &'a Alignment<T>,
    index: usize,
    size_hint: usize,
}

impl<'a, T> Iterator for AlignmentPositionIterator<'a, T>
where
    T: Clone,
{
    type Item = Vec<&'a T>;
    fn next(&mut self) -> Option<Vec<&'a T>> {
        if self.index >= self.alignment.length {
            return None;
        }
        match self.alignment.nth_position(self.index) {
            Some(position) => {
                self.index = self.index.saturating_add(1);
                self.size_hint = self.size_hint.saturating_sub(1);
                Some(position)
            }
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.size_hint < usize::max_value() {
            // ?
            (self.size_hint, Some(self.size_hint))
        } else {
            (usize::max_value(), None)
        }
    }
}

impl<'a, T> ExactSizeIterator for AlignmentPositionIterator<'a, T>
where
    T: Clone,
{
    fn len(&self) -> usize {
        let (lower, upper) = self.size_hint();
        // Note: This assertion is overly defensive, but it checks the invariant
        // guaranteed by the trait. If this trait were rust-internal,
        // we could use debug_assert!; assert_eq! will check all Rust user
        // implementations too.
        assert_eq!(upper, Some(lower));
        lower
    }
}

struct AlignmentSequenceIterator<'a, T> {
    alignment: &'a Alignment<T>,
    index: usize,
    size_hint: usize,
}

impl<'a, T> Iterator for AlignmentSequenceIterator<'a, T>
where
    T: Clone,
{
    type Item = Vec<&'a T>;
    fn next(&mut self) -> Option<Vec<&'a T>> {
        if self.index >= self.alignment.n_sequences {
            return None;
        }

        match self.alignment.nth_sequence(self.index) {
            Some(seq) => {
                self.index = self.index.saturating_add(1);
                self.size_hint = self.size_hint.saturating_sub(1);
                Some(seq)
            }
            None => None,
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.alignment.nth_sequence(n)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.size_hint < usize::max_value() {
            // ?
            (self.size_hint, Some(self.size_hint))
        } else {
            (usize::max_value(), None)
        }
    }
}

impl<'a, T> ExactSizeIterator for AlignmentSequenceIterator<'a, T>
where
    T: Clone,
{
    fn len(&self) -> usize {
        let (lower, upper) = self.size_hint();
        // Note: This assertion is overly defensive, but it checks the invariant
        // guaranteed by the trait. If this trait were rust-internal,
        // we could use debug_assert!; assert_eq! will check all Rust user
        // implementations too.
        assert_eq!(upper, Some(lower));
        lower
    }
}

impl<T> Alignment<T> {
    /// Returns a vector of sequence names
    #[must_use]
    pub const fn names(&self) -> &Vec<String> {
        &self.names
    }

    /// Returns a vector of sequence descriptions
    #[must_use]
    pub const fn descriptions(&self) -> &Vec<String> {
        &self.descriptions
    }

    /// Returns the fixed `length` of the Alignment `self`
    #[must_use]
    pub const fn length(&self) -> &usize {
        &self.length
    }

    /// Returns the number of sequences contained in `self`
    #[must_use]
    pub const fn n_sequences(&self) -> &usize {
        &self.n_sequences
    }

    /// Returns an Iterator over the positions of the alignment
    pub fn iter_positions(
        &self,
    ) -> impl Iterator<Item = Vec<&T>> + ExactSizeIterator<Item = Vec<&T>>
    where
        T: Clone,
    {
        AlignmentPositionIterator {
            alignment: self,
            index: 0_usize,
            size_hint: self.length,
        }
    }

    /// Returns an Iterator over the sequences of the alignment
    pub fn iter_sequences(
        &self,
    ) -> impl Iterator<Item = Vec<&T>> + ExactSizeIterator<Item = Vec<&T>>
    where
        T: Clone,
    {
        AlignmentSequenceIterator {
            alignment: self,
            index: 0_usize,
            size_hint: self.n_sequences,
        }
    }

    /// Returns an empty `Alignment` of fixed `length`
    ///
    ///
    /// # Examples
    ///
    /// ```rust
    ///  # use multi_seq_align::Alignment;
    ///  let alignment = Alignment::<char>::new(42);
    ///
    ///  assert_eq!(*alignment.length(), 42 as usize);
    ///  assert_eq!(*alignment.n_sequences(), 0 as usize);
    /// ```
    #[must_use]
    pub const fn new(length: usize) -> Self {
        Self {
            names: Vec::new(),
            descriptions: Vec::new(),
            sequences: Vec::new(),
            n_sequences: 0_usize,
            length,
        }
    }

    /// Returns `true` if `self` doesn't contains any sequence
    ///
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use multi_seq_align::Alignment;
    /// let alignment = Alignment::<char>::new(42);
    /// println!("\n\n>>> {:?} \n^^^^", alignment);
    /// assert!(alignment.is_empty())
    ///```
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.n_sequences == 0_usize
    }

    /// Create an `Alignment` from same length vectors of names, descriptions, sequences
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use multi_seq_align::Alignment;
    /// let align = Alignment::create(
    ///     vec![
    ///         "ELMI001940".to_string(),
    ///         "ELMI001939".to_string(),
    ///         "ELMI001938".to_string(),
    ///     ],
    ///     vec!["".to_string(), "".to_string(), "".to_string()],
    ///     &[
    ///         b"AVEQTPRK".to_vec(),
    ///         b"SVEQTPRK".to_vec(),
    ///         b"SVEQTPKK".to_vec(),
    ///     ],
    /// )
    /// .unwrap();
    ///
    /// assert_eq!(*align.length(), 8);
    /// assert_eq!(*align.n_sequences(), 3);
    /// ```
    ///
    /// # Errors
    ///
    /// Will return an error if `names`, `descriptions` and `sequences` have different lengths, and also if the sequences have different lengths (based on the first sequence).
    pub fn create(
        names: Vec<String>,
        descriptions: Vec<String>,
        sequences: &[Vec<T>],
    ) -> Result<Self, MultiSeqAlignError>
    where
        T: Clone,
    {
        debug_assert!(names.len() == descriptions.len() && names.len() == sequences.len());

        let length = utils::first_sequence_length(sequences);
        utils::check_unequal_lengths(sequences, &names, length)?;

        let n_sequences = sequences.len();

        let sequences_vec = sequences.iter().flat_map(|x| x.to_vec()).collect();

        Ok(Self {
            names,
            descriptions,
            sequences: sequences_vec,
            n_sequences,
            length,
        })
    }

    /// Add a sequence to `self`
    ///
    /// The new sequence must have the same length than `self.length`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use multi_seq_align::Alignment;
    /// let mut align = Alignment::new(8);
    ///
    ///  assert_eq!(*align.n_sequences(), 0);
    ///
    /// align
    ///     .add_aligned_sequence("ELMI001940".to_string(), "".to_string(), b"AVEQTPRK".to_vec())
    ///     .unwrap();
    ///
    /// assert_eq!(*align.n_sequences(), 1);
    ///
    /// align
    ///     .add_aligned_sequence("ELMI001939".to_string(), "".to_string(), b"SVEQTPRK".to_vec())
    ///     .unwrap();
    ///
    /// assert_eq!(*align.n_sequences(), 2);
    /// ```
    ///
    /// # Errors
    ///
    /// Will return an error if the length of `sequence` is different from the one of the alignment.
    pub fn add_aligned_sequence<'a>(
        &'a mut self,
        name: String,
        description: String,
        sequence: Vec<T>,
    ) -> Result<&'a mut Self, MultiSeqAlignError> {
        if sequence.len() != self.length {
            return Err(MultiSeqAlignError::NewSequenceOfDifferentLength {
                expected_length: self.length,
                sequences_name: name,
                found_length: sequence.len(),
            });
        }

        self.names.push(name);

        self.descriptions.push(description);

        self.sequences.extend(sequence);

        self.n_sequences += 1;

        Ok(self)
    }

    /// Returns all amino acids / bases at a `position` in the alignment `self`. The returned vector has a length equal of number of sequences in `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use multi_seq_align::Alignment;
    /// let align = Alignment::<u8>::create(
    ///     vec!["testname1".to_string(), "testname2".to_string()],
    ///     vec!["desc1".to_string(), "desc2".to_string()],
    ///     &[b"ELK".to_vec(), b"ILK".to_vec()],
    /// )
    /// .unwrap();
    ///
    /// assert_eq!(align.nth_position(0).unwrap(), &[&b'E', &b'I']);
    /// ```
    /// # Panics
    ///
    /// Panics if `n` is greater or equal to the `length` of the Alignment.
    #[must_use]
    pub fn nth_position(&self, n: usize) -> Option<Vec<&T>> {
        assert!(n < self.length);
        (0..self.n_sequences)
            .map(|i| self.sequences.get(i * self.length + n))
            .collect::<Vec<Option<&T>>>()
            .into_iter()
            .collect::<Option<Vec<&T>>>()
    }

    /// Returns all amino acids / bases of the sequence at the `index` of the Alignment `self`. The returned vector has a length equal to the length of the Alignment `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use multi_seq_align::Alignment;
    /// let align = Alignment::<u8>::create(
    ///     vec!["testname1".to_string(), "testname2".to_string()],
    ///     vec!["desc1".to_string(), "desc2".to_string()],
    ///     &[b"ELK".to_vec(), b"ILK".to_vec()],
    /// )
    /// .unwrap();
    ///
    /// assert_eq!(align.nth_sequence(1).unwrap(), &[&b'I', &b'L', &b'K']);
    /// ```
    /// # Panics
    ///
    /// Panics if `index` is greater or equal to the `n_sequences` of the Alignment.
    #[must_use]
    pub fn nth_sequence(&self, index: usize) -> Option<Vec<&T>> {
        debug_assert!(index < self.n_sequences);

        (0..self.length)
            .map(|i| self.sequences.get(self.length * index + i))
            .collect::<Vec<Option<&T>>>()
            .into_iter()
            .collect::<Option<Vec<&T>>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn new_align() {
        let x = Alignment::<char>::new(5_usize);
        assert!(x.names.is_empty());
        assert!(x.sequences.is_empty());
        assert_eq!(x.length, 5_usize);
        assert_eq!(x.n_sequences, 0_usize);
    }

    #[test]
    fn new_alignment_with_desc() {
        let x = Alignment::<u8>::create(
            vec!["testname1".to_string(), "testname2".to_string()],
            vec!["desc1".to_string(), "desc2".to_string()],
            &[b"ELK".to_vec(), b"ILK".to_vec()],
        )
        .unwrap();
        assert_eq!(
            x.names,
            vec!["testname1".to_string(), "testname2".to_string()]
        );
        assert_eq!(
            x.descriptions,
            vec!["desc1".to_string(), "desc2".to_string()]
        );
        assert_eq!(x.sequences, vec![b'E', b'L', b'K', b'I', b'L', b'K']);
        assert_eq!(x.length, 3);
        assert_eq!(x.n_sequences, 2);
    }

    #[test]
    fn add_1_sequence() {
        let mut align = Alignment::create(
            vec![String::from("NAME1"), String::from("NAME2")],
            vec![String::from("desc1"), String::from("desc2")],
            &[b"ALKHITAN".to_vec(), b"VLK-ITAN".to_vec()],
        )
        .unwrap();

        align
            .add_aligned_sequence("added1".to_string(), "".to_string(), b"ALRYITAT".to_vec())
            .unwrap();

        assert_eq!(align.n_sequences, 3_usize);
        assert_eq!(align.nth_position(3).unwrap(), vec![&b'H', &b'-', &b'Y'])
    }

    #[test]
    fn add_1_sequence_wrong_length() {
        let mut x = Alignment::new(3_usize);
        let error = x
            .add_aligned_sequence(
                String::from("too_long"),
                "add sequence of length 5 to an alignment of length 3".to_string(),
                b"ILKAV".to_vec(),
            )
            .err()
            .unwrap();
        let expected = MultiSeqAlignError::NewSequenceOfDifferentLength {
            expected_length: 3_usize,
            sequences_name: String::from("too_long"),
            found_length: 5_usize,
        };
        assert_eq!(error, expected);
    }

    #[test]
    fn add_to_new() {
        let mut x = Alignment::new(3_usize);

        x.add_aligned_sequence("sequence1".to_string(), "".to_string(), b"ELK".to_vec())
            .unwrap();
        assert_eq!(x.n_sequences, 1_usize);
        assert_eq!(x.length, 3_usize);
        assert_eq!(x.names.len(), 1_usize);
        assert_eq!(x.descriptions.len(), 1_usize);
        assert_eq!(x.sequences.len(), 3_usize);

        x.add_aligned_sequence("sequence2".to_string(), "".to_string(), b"ILK".to_vec())
            .unwrap();

        assert_eq!(x.n_sequences, 2_usize);
        assert_eq!(x.length, 3_usize);
        assert_eq!(x.names.len(), 2_usize);
        assert_eq!(x.descriptions.len(), 2_usize);
        assert_eq!(x.sequences.len(), 6_usize);
    }

    #[test]
    fn empty_align() {
        let mut x = Alignment::new(3_usize);
        assert!(x.is_empty());
        x.add_aligned_sequence(String::from("sequence1"), "".to_string(), b"ILK".to_vec())
            .unwrap();
        assert!(!x.is_empty());
    }

    #[test]
    fn nth_residues_3() {
        let align = Alignment::create(
            vec![String::from("NAME1"), String::from("NAME2")],
            vec![String::from("desc1"), String::from("desc2")],
            &[b"ALKHITAN".to_vec(), b"VLK-ITAN".to_vec()],
        )
        .unwrap();
        assert_eq!(align.nth_position(3).unwrap(), vec![&b'H', &b'-'])
    }

    #[test]
    fn nth_residues_more_seqs() {
        let align = Alignment::create(
            vec![
                "seq1".to_string(),
                "seq2".to_string(),
                "seq3".to_string(),
                "seq4".to_string(),
            ],
            vec!["".to_string(); 4],
            &[
                b"ALKHITAN".to_vec(),
                b"VLK-ITAN".to_vec(),
                b"ALKWITAN".to_vec(),
                b"VLKMITAN".to_vec(),
            ],
        )
        .unwrap();
        assert_eq!(
            align.nth_position(3).unwrap(),
            vec![&b'H', &b'-', &b'W', &b'M']
        )
    }

    #[test]
    #[should_panic(expected = "assertion failed: n < self.length")]
    fn nth_residues_out() {
        let align = Alignment::create(
            vec![String::from("NAME1"), String::from("NAME2")],
            vec![String::from("desc1"), String::from("desc2")],
            &[b"ALKHITAN".to_vec(), b"VLK-ITAN".to_vec()],
        )
        .unwrap();
        let _out_of_bonds = align.nth_position(10);
    }

    #[test]
    fn different_seq_lengths() {
        let error = Alignment::create(
            vec![String::from("NAME1"), String::from("NAME2")],
            vec![String::from("desc1"), String::from("desc2")],
            &[b"ALKHITAN".to_vec(), b"VLK-ITAN---".to_vec()],
        )
        .err()
        .unwrap();

        let expected = MultiSeqAlignError::MultipleSequencesOfDifferentLengths {
            expected_length: 8,
            sequences_names: vec![String::from("NAME2")],
            found_lengths: vec![11],
        };
        assert_eq!(error, expected);
    }

    #[test]
    fn for_names() {
        let align = Alignment::<u8>::create(
            vec!["NAME1".to_string(), "NAME2".to_string()],
            vec!["desc1".to_string(), "desc2".to_string()],
            &[b"ELK".to_vec(), b"ILK".to_vec()],
        )
        .unwrap();

        let mut x: Vec<&String> = Vec::new();

        for name in align.names() {
            x.push(name);
        }

        assert_eq!(x, vec!["NAME1", "NAME2"]);
    }

    #[test]
    fn for_positions() {
        let align = Alignment::create(
            vec![String::from("NAME1"), String::from("NAME2")],
            vec![String::from("desc1"), String::from("desc2")],
            &[b"ALKHITAN".to_vec(), b"VLK-ITAN".to_vec()],
        )
        .unwrap();

        let mut x = Vec::new();

        for col in align.iter_positions() {
            x.push(col);
        }

        assert_eq!(x.len(), 8);
        assert_eq!(x.get(0).unwrap(), &[&b'A', &b'V']);
        assert_eq!(x.get(3).unwrap(), &[&b'H', &b'-']);
    }

    #[test]
    #[should_panic]
    fn for_positions_out_of_bonds() {
        let align = Alignment::create(
            vec![String::from("NAME1"), String::from("NAME2")],
            vec![String::from("desc1"), String::from("desc2")],
            &[b"ALKHITAN".to_vec(), b"VLK-ITAN".to_vec()],
        )
        .unwrap();
        let mut x = Vec::new();

        for col in align.iter_positions() {
            x.push(col);
        }

        let _ = x.get(22).unwrap();
    }

    #[test]
    fn for_positions_exact() {
        let align = Alignment::create(
            vec![String::from("NAME1"), String::from("NAME2")],
            vec![String::from("desc1"), String::from("desc2")],
            &[b"ALKHITAN".to_vec(), b"VLK-ITAN".to_vec()],
        )
        .unwrap();

        assert_eq!(align.iter_positions().len(), 8);
        assert_eq!(align.iter_positions().next().unwrap().len(), 2);
    }

    #[test]
    fn for_sequences() {
        let align = Alignment::create(
            vec![String::from("NAME1"), String::from("NAME2")],
            vec![String::from("desc1"), String::from("desc2")],
            &[b"ALKHITAN".to_vec(), b"VLK-ITAN".to_vec()],
        )
        .unwrap();

        let mut x = Vec::new();

        for row in align.iter_sequences() {
            assert_eq!(row.len(), 8);
            x.push(row);
        }

        assert_eq!(x.len(), 2)
    }

    #[test]
    fn for_sequences_exact() {
        let align = Alignment::create(
            vec![String::from("NAME1"), String::from("NAME2")],
            vec![String::from("desc1"), String::from("desc2")],
            &[b"ALKHITAN".to_vec(), b"VLK-ITAN".to_vec()],
        )
        .unwrap();

        assert_eq!(align.iter_sequences().len(), 2);
        assert_eq!(align.iter_sequences().next().unwrap().len(), 8);
    }

    #[test]
    fn for_sequences_collect() {
        let align = Alignment::create(
            vec![String::from("NAME1"), String::from("NAME2")],
            vec![String::from("desc1"), String::from("desc2")],
            &[b"ALKHITAN".to_vec(), b"VLK-ITAN".to_vec()],
        )
        .unwrap();

        assert_eq!(align.iter_sequences().len(), 2);
        assert_eq!(align.iter_sequences().next().unwrap().len(), 8);
    }
}
