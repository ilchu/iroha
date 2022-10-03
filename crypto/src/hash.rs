#[cfg(not(feature = "std"))]
use alloc::{alloc::alloc, format, string::String, vec, vec::Vec};
use core::{hash, marker::PhantomData, num::NonZeroU8};

use derive_more::{DebugCustom, Deref, DerefMut, Display};
use iroha_schema::prelude::*;
use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use ursa::blake2::{
    digest::{Update, VariableOutput},
    VarBlake2b,
};

use crate::ffi;

ffi::ffi_item! {
    /// Hash of Iroha entities. Currently supports only blake2b-32.
    /// The least significant bit of hash is set to 1.
    #[derive(
        Clone,
        Copy,
        Display,
        DebugCustom,
        Hash,
        Eq,
        PartialEq,
        Ord,
        PartialOrd,
        IntoSchema,
    )]
    #[display(fmt = "{}", "hex::encode(self.as_ref())")]
    #[debug(fmt = "{}", "hex::encode(self.as_ref())")]
    #[repr(C)]
    pub struct Hash {
        more_significant_bits: [u8; Self::LENGTH - 1],
        least_significant_byte: NonZeroU8,
    }
}

// NOTE: Hash is FFI serialized as an array (a pointer in a function call, by value when part of a struct)
iroha_ffi::ffi_type! {unsafe impl Transparent for Hash[[u8; Hash::LENGTH]] validated with {Hash::is_lsb_1} }

impl iroha_ffi::option::Niche for Hash {
    // NOTE: Any value that has lsb=0 is a niche value
    const NICHE_VALUE: Self::ReprC = [0; Hash::LENGTH];
}

impl Hash {
    /// Length of hash
    pub const LENGTH: usize = 32;

    /// Wrap the given bytes; they must be prehashed with `VarBlake2b`
    pub fn prehashed(mut hash: [u8; Self::LENGTH]) -> Self {
        hash[Self::LENGTH - 1] |= 1;
        #[allow(unsafe_code)]
        // SAFETY:
        // - any `u8` value after bitwise or with 1 will be at least 1
        // - `Hash` and `[u8; Hash::LENGTH]` have the same memory layout
        unsafe {
            core::mem::transmute(hash)
        }
    }

    /// Hash the given bytes.
    #[cfg(feature = "std")]
    #[allow(clippy::expect_used)]
    #[must_use]
    pub fn new(bytes: impl AsRef<[u8]>) -> Self {
        let vec_hash = VarBlake2b::new(Self::LENGTH)
            .expect("Failed to initialize variable size hash")
            .chain(bytes)
            .finalize_boxed();
        let mut hash = [0; Self::LENGTH];
        hash.copy_from_slice(&vec_hash);
        Hash::prehashed(hash)
    }

    /// Adds type information to the hash. Be careful about using this function
    /// since it is not possible to validate the correctness of the conversion.
    /// Prefer creating new hashes with [`HashOf::new`] whenever possible
    #[must_use]
    pub const fn typed<T>(self) -> HashOf<T> {
        HashOf(self, PhantomData)
    }

    /// Check if least significant bit of `[u8; Hash::LENGTH]` is 1
    fn is_lsb_1(hash: &[u8; Self::LENGTH]) -> bool {
        hash[Self::LENGTH - 1] & 1 == 1
    }
}

impl From<Hash> for [u8; Hash::LENGTH] {
    #[inline]
    fn from(hash: Hash) -> Self {
        #[allow(unsafe_code)]
        // SAFETY: `Hash` and `[u8; Hash::LENGTH]` have the same memory layout
        unsafe {
            core::mem::transmute(hash)
        }
    }
}

impl AsRef<[u8; Hash::LENGTH]> for Hash {
    #[inline]
    fn as_ref(&self) -> &[u8; Hash::LENGTH] {
        #[allow(unsafe_code, trivial_casts)]
        // SAFETY: `Hash` and `[u8; Hash::LENGTH]` have the same memory layout
        unsafe {
            &*((self as *const Self).cast::<[u8; Self::LENGTH]>())
        }
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hash: &[u8; Self::LENGTH] = self.as_ref();
        hash.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        <[u8; Self::LENGTH]>::deserialize(deserializer)
            .and_then(|hash| {
                Hash::is_lsb_1(&hash)
                    .then_some(hash)
                    .ok_or_else(|| D::Error::custom("expect least significant bit of hash to be 1"))
            })
            .map(Self::prehashed)
    }
}

impl Encode for Hash {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        f(self.as_ref())
    }
}

impl Decode for Hash {
    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        <[u8; Self::LENGTH]>::decode(input)
            .and_then(|hash| {
                Hash::is_lsb_1(&hash)
                    .then_some(hash)
                    .ok_or_else(|| "expect least significant bit of hash to be 1".into())
            })
            .map(Self::prehashed)
    }
}

impl<T> From<HashOf<T>> for Hash {
    fn from(HashOf(hash, _): HashOf<T>) -> Self {
        hash
    }
}

/// Represents hash of Iroha entities like `Block` or `Transaction`. Currently supports only blake2b-32.
// Lint triggers when expanding #[codec(skip)]
#[allow(clippy::default_trait_access)]
#[derive(DebugCustom, Deref, DerefMut, Display, Decode, Encode, Deserialize, Serialize)]
#[display(fmt = "{}", _0)]
#[debug(fmt = "{{ {} {_0} }}", "core::any::type_name::<Self>()")]
#[serde(transparent)]
#[repr(transparent)]
pub struct HashOf<T>(
    #[deref]
    #[deref_mut]
    Hash,
    #[codec(skip)] PhantomData<T>,
);

impl<T> Clone for HashOf<T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}
impl<T> Copy for HashOf<T> {}

impl<T> PartialEq for HashOf<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}
impl<T> Eq for HashOf<T> {}

impl<T> PartialOrd for HashOf<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl<T> Ord for HashOf<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T> hash::Hash for HashOf<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<T> AsRef<[u8; Hash::LENGTH]> for HashOf<T> {
    fn as_ref(&self) -> &[u8; Hash::LENGTH] {
        self.0.as_ref()
    }
}

impl<T> HashOf<T> {
    /// Transmutes hash to some specific type.
    /// Don't use this method if not required.
    #[inline]
    #[must_use]
    pub const fn transmute<F>(self) -> HashOf<F> {
        HashOf(self.0, PhantomData)
    }
}

impl<T: Encode> HashOf<T> {
    /// Construct typed hash
    #[cfg(feature = "std")]
    #[must_use]
    pub fn new(value: &T) -> Self {
        Self(Hash::new(value.encode()), PhantomData)
    }
}

impl<T: IntoSchema> IntoSchema for HashOf<T> {
    fn type_name() -> String {
        format!("{}::HashOf<{}>", module_path!(), T::type_name())
    }
    fn schema(map: &mut MetaMap) {
        map.entry(Self::type_name()).or_insert_with(|| {
            Metadata::Tuple(UnnamedFieldsMeta {
                types: vec![Hash::type_name()],
            })
        });

        Hash::schema(map);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::restriction)]

    #[cfg(feature = "std")]
    use hex_literal::hex;

    #[cfg(feature = "std")]
    use super::*;

    #[test]
    #[cfg(feature = "std")]
    fn blake2_32b() {
        let mut hasher = VarBlake2b::new(32).unwrap();
        hasher.update(hex!("6920616d2064617461"));
        hasher.finalize_variable(|res| {
            assert_eq!(
                res[..],
                hex!("ba67336efd6a3df3a70eeb757860763036785c182ff4cf587541a0068d09f5b2")[..]
            );
        })
    }
}
