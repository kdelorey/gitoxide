//! Contains a borrowed Object bound to a buffer holding its decompressed data.

use crate::pack;
use git_object::immutable;

/// A borrowed object using a borrowed slice as backing buffer.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Object<'a> {
    /// kind of object
    pub kind: git_object::Kind,
    /// decoded, decompressed data, owned by a backing store.
    pub data: &'a [u8],
    /// If `Some`, this object is from a pack whose pack location can be used to look up pack related information
    pub(crate) pack_location: Option<pack::bundle::Location>,
}

impl<'a> Object<'a> {
    /// Decodes the data in the backing slice into a [`git_object::immutable::Object`], allowing to access all of its data
    /// conveniently. The cost of parsing an object is negligible.
    ///
    /// **Note** that [mutable, decoded objects][git_object::mutable::Object] can be created from a [`crate::data::Object`]
    /// using [`git_object::immutable::Object::into_mutable()`].
    pub fn decode(&self) -> Result<immutable::Object<'a>, immutable::object::decode::Error> {
        Ok(match self.kind {
            git_object::Kind::Tree => immutable::Object::Tree(immutable::Tree::from_bytes(self.data)?),
            git_object::Kind::Blob => immutable::Object::Blob(immutable::Blob { data: self.data }),
            git_object::Kind::Commit => immutable::Object::Commit(immutable::Commit::from_bytes(self.data)?),
            git_object::Kind::Tag => immutable::Object::Tag(immutable::Tag::from_bytes(self.data)?),
        })
    }
}

/// Types supporting object hash verification
pub mod verify {
    use crate::{hash, loose};
    use std::io;

    /// Returned by [`crate::data::Object::verify_checksum()`]
    #[derive(thiserror::Error, Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Object expected to have id {desired}, but actual id was {actual}")]
        ChecksumMismatch {
            desired: git_hash::ObjectId,
            actual: git_hash::ObjectId,
        },
    }

    impl crate::data::Object<'_> {
        /// Compute the checksum of `self` and compare it with the `desired` hash.
        /// If the hashes do not match, an [`Error`] is returned, containing the actual
        /// hash of `self`.
        pub fn verify_checksum(&self, desired: impl AsRef<git_hash::oid>) -> Result<(), Error> {
            let desired = desired.as_ref();
            let mut sink = hash::Write::new(io::sink(), desired.kind());

            loose::object::header::encode(self.kind, self.data.len() as u64, &mut sink).expect("hash to always work");
            sink.hash.update(&self.data);

            let actual_id = git_hash::ObjectId::from(sink.hash.digest());
            if desired != actual_id {
                return Err(Error::ChecksumMismatch {
                    desired: desired.into(),
                    actual: actual_id,
                });
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_of_object() {
        assert_eq!(std::mem::size_of::<Object<'_>>(), 48, "this shouldn't change unnoticed");
    }
}
