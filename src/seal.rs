#[cfg(target_os = "linux")]
const SEAL_MASK: u32 = (libc::F_SEAL_SEAL | libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE | libc::F_SEAL_FUTURE_WRITE) as u32;

#[cfg(not(target_os = "linux"))]
const SEAL_MASK: u32 = (libc::F_SEAL_SEAL | libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE) as u32;

#[cfg(target_os = "linux")]
const ALL_SEALS: [Seal; 5] = [
	Seal::Seal,
	Seal::Shrink,
	Seal::Grow,
	Seal::Write,
	Seal::FutureWrite,
];

#[cfg(not(target_os = "linux"))]
const ALL_SEALS: [Seal; 4] = [
	Seal::Seal,
	Seal::Shrink,
	Seal::Grow,
	Seal::Write,
];

/// A seal that prevents certain actions from being performed on a file.
///
/// Note that seals apply to a file, not a file descriptor.
/// If two file descriptors refer to the same file, they share the same set of seals.
///
/// Seals can not be removed from a file once applied.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
#[repr(u32)]
#[non_exhaustive]
pub enum Seal {
	/// Prevent adding more seals to the file.
	Seal = libc::F_SEAL_SEAL as u32,

	/// Prevent the file from being shrunk with `truncate` or similar.
	///
	/// Combine with [`Seal::Grow`] to prevent the file from being resized in any way.
	Shrink = libc::F_SEAL_SHRINK as u32,

	/// Prevent the file from being extended with `truncate`, `fallocate` or simillar.
	///
	/// Combine with [`Seal::Shrink`] to prevent the file from being resized in any way.
	Grow = libc::F_SEAL_GROW as u32,

	/// Prevent write to the file.
	///
	/// This will block *all* writes to the file and prevents any shared, writable memory mappings from being created.
	///
	/// If a shared, writable memory mapping already exists, adding this seal will fail.
	Write = libc::F_SEAL_WRITE as u32,

	/// Similar to [`Seal::Write`], but allows existing shared, writable memory mappings to modify the file contents.
	///
	/// This can be used to share a read-only view of the file with other processes,
	/// while still being able to modify the contents through an existing mapping.
	#[cfg(target_os = "linux")]
	FutureWrite = libc::F_SEAL_FUTURE_WRITE as u32,
}

/// A set of [seals][Seal].
#[derive(Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Seals {
	bits: u32,
}

impl Seals {
	/// Construct a set of seals from a bitmask.
	///
	/// Unknown bits are trunctated.
	#[inline]
	pub const fn from_bits_truncate(bits: u32) -> Self {
		Self::from_bits(bits & SEAL_MASK)
	}

	/// Construct a set of seals from a bitmask.
	///
	/// Unknown bits are trunctated.
	#[inline]
	const fn from_bits(bits: u32) -> Self {
		Self { bits }
	}

	#[inline]
	pub const fn bits(self) -> u32 {
		self.bits
	}

	/// Get an empty set of seals.
	#[inline]
	pub const fn empty() -> Self {
		Self::from_bits_truncate(0)
	}

	/// Get a set of seals containing all possible seals.
	#[inline]
	pub const fn all() -> Self {
		Self::from_bits(SEAL_MASK)
	}

	/// Get the number of seals in the set.
	#[inline]
	pub const fn len(self) -> usize {
		self.bits.count_ones() as usize
	}

	/// Check if the set of seals is empty.
	#[inline]
	pub const fn is_empty(self) -> bool {
		self.bits == 0
	}

	/// Check if the set of seals contains all possible seals.
	#[inline]
	pub const fn is_all(self) -> bool {
		self.bits == Self::all().bits
	}

	/// Check if the set of seals contains all the given seals.
	#[inline]
	pub fn contains(self, other: impl Into<Self>) -> bool {
		let other = other.into();
		self & other == other
	}

	/// Check if the set of seals contains at least one of the given seals.
	#[inline]
	pub fn intersects(self, other: impl Into<Self>) -> bool {
		!(self & other).is_empty()
	}

	/// Iterate over the seals in the set.
	#[inline]
	pub fn iter(&self) -> SealsIterator {
		SealsIterator::new(*self)
	}
}

impl IntoIterator for Seals {
	type Item = Seal;
	type IntoIter = SealsIterator;

	#[inline]
	fn into_iter(self) -> SealsIterator {
		self.iter()
	}
}

impl IntoIterator for &Seals {
	type Item = Seal;
	type IntoIter = SealsIterator;

	#[inline]
	fn into_iter(self) -> SealsIterator {
		self.iter()
	}
}

impl From<Seal> for Seals {
	#[inline]
	fn from(other: Seal) -> Self {
		Self::from_bits_truncate(other as u32)
	}
}

impl<T: Into<Seals>> std::ops::BitOr<T> for Seals {
	type Output = Seals;

	#[inline]
	fn bitor(self, right: T) -> Self {
		Self::from_bits(self.bits | right.into().bits)
	}
}

impl<T: Into<Seals>> std::ops::BitOrAssign<T> for Seals {
	#[inline]
	fn bitor_assign(&mut self, right: T) {
		self.bits |= right.into().bits;
	}
}

impl<T: Into<Seals>> std::ops::BitAnd<T> for Seals {
	type Output = Seals;

	#[inline]
	fn bitand(self, right: T) -> Self {
		Self::from_bits(self.bits & right.into().bits)
	}
}

impl<T: Into<Seals>> std::ops::BitAndAssign<T> for Seals {
	#[inline]
	fn bitand_assign(&mut self, right: T) {
		self.bits &= right.into().bits;
	}
}

impl<T: Into<Seals>> std::ops::Sub<T> for Seals {
	type Output = Seals;

	#[inline]
	fn sub(self, right: T) -> Self {
		Self::from_bits(self.bits & !right.into().bits)
	}
}

impl<T: Into<Seals>> std::ops::SubAssign<T> for Seals {
	#[inline]
	fn sub_assign(&mut self, right: T) {
		self.bits &= !right.into().bits;
	}
}

impl<T: Into<Seals>> std::ops::BitXor<T> for Seals {
	type Output = Seals;

	#[inline]
	fn bitxor(self, right: T) -> Self {
		Self::from_bits(self.bits ^ right.into().bits)
	}
}

impl<T: Into<Seals>> std::ops::BitXorAssign<T> for Seals {
	#[inline]
	fn bitxor_assign(&mut self, right: T) {
		self.bits ^= right.into().bits;
	}
}

impl std::ops::Not for Seals {
	type Output = Seals;

	#[inline]
	fn not(self) -> Seals {
		Self::from_bits(!self.bits)
	}
}

impl std::ops::BitOr<Seals> for Seal {
	type Output = Seals;

	#[inline]
	fn bitor(self, right: Seals) -> Seals {
		Seals::from(self) | right
	}
}

impl std::ops::BitAnd<Seals> for Seal {
	type Output = Seals;

	#[inline]
	fn bitand(self, right: Seals) -> Seals {
		Seals::from(self) & right
	}
}

impl std::ops::Sub<Seals> for Seal {
	type Output = Seals;

	#[inline]
	fn sub(self, right: Seals) -> Seals {
		Seals::from(self) - right
	}
}

impl std::ops::BitXor<Seals> for Seal {
	type Output = Seals;

	#[inline]
	fn bitxor(self, right: Seals) -> Seals {
		Seals::from(self) ^ right
	}
}

impl std::ops::BitOr<Seal> for Seal {
	type Output = Seals;

	#[inline]
	fn bitor(self, right: Seal) -> Seals {
		Seals::from(self) | right
	}
}

impl std::ops::BitAnd<Seal> for Seal {
	type Output = Seals;

	#[inline]
	fn bitand(self, right: Seal) -> Seals {
		Seals::from(self) & right
	}
}

impl std::ops::Sub<Seal> for Seal {
	type Output = Seals;

	#[inline]
	fn sub(self, right: Seal) -> Seals {
		Seals::from(self) - right
	}
}

impl std::ops::BitXor<Seal> for Seal {
	type Output = Seals;

	#[inline]
	fn bitxor(self, right: Seal) -> Seals {
		Seals::from(self) ^ right
	}
}

pub struct SealsIterator {
	seals: Seals,
}

impl SealsIterator {
	fn new(seals: Seals) -> Self {
		Self { seals }
	}

}

impl Iterator for SealsIterator {
	type Item = Seal;

	#[inline]
	fn next(&mut self) -> Option<Seal> {
		for &seal in &ALL_SEALS {
			if self.seals.contains(seal) {
				self.seals -= seal;
				return Some(seal)
			}
		}
		None
	}
}

impl std::fmt::Debug for Seals {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Seals {{ ")?;
		for (i, seal) in self.iter().enumerate() {
			if i == 0 {
				write!(f, "{:?} ", seal)?
			} else {
				write!(f, "| {:?} ", seal)?
			}
		}
		write!(f, "}}")?;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	#[test]
	fn test_empty() {
		assert!(Seals::empty().len() == 0);
		assert!(Seals::empty().is_empty());
		assert!(!Seals::empty().is_all());
		assert!(Seals::empty().contains(Seals::empty()));
		assert!(!Seals::empty().contains(Seals::all()));
		assert!(!Seals::empty().contains(Seal::Seal));
		assert!(!Seals::empty().contains(Seal::Shrink));
		assert!(!Seals::empty().contains(Seal::Grow));
		assert!(!Seals::empty().contains(Seal::Write));
		assert!(!Seals::empty().contains(Seal::FutureWrite));
	}

	#[test]
	fn test_all() {
		assert!(Seals::all().len() == 5);
		assert!(!Seals::all().is_empty());
		assert!(Seals::all().is_all());
		assert!(Seals::all().contains(Seals::empty()));
		assert!(Seals::all().contains(Seals::all()));
		assert!(Seals::all().contains(Seal::Seal));
		assert!(Seals::all().contains(Seal::Shrink));
		assert!(Seals::all().contains(Seal::Grow));
		assert!(Seals::all().contains(Seal::Write));
		assert!(Seals::all().contains(Seal::FutureWrite));
	}

	#[test]
	fn test_iter() {
		let mut iter = Seals::all().into_iter();
		assert!(iter.next() == Some(Seal::Seal));
		assert!(iter.next() == Some(Seal::Shrink));
		assert!(iter.next() == Some(Seal::Grow));
		assert!(iter.next() == Some(Seal::Write));
		assert!(iter.next() == Some(Seal::FutureWrite));
		assert!(iter.next() == None);

		let mut iter = (Seal::Shrink | Seal::Grow).into_iter();
		assert!(iter.next() == Some(Seal::Shrink));
		assert!(iter.next() == Some(Seal::Grow));
		assert!(iter.next() == None);
	}

	#[test]
	fn test_bitor() {
		assert!((Seal::Seal | Seal::FutureWrite | Seal::Write).len() == 3);
		assert!((Seal::Seal | Seal::FutureWrite | Seal::Write).contains(Seal::Seal));
		assert!(!(Seal::Seal | Seal::FutureWrite | Seal::Write).contains(Seal::Shrink));
		assert!(!(Seal::Seal | Seal::FutureWrite | Seal::Write).contains(Seal::Grow));
		assert!((Seal::Seal | Seal::FutureWrite | Seal::Write).contains(Seal::Write));
		assert!((Seal::Seal | Seal::FutureWrite | Seal::Write).contains(Seal::FutureWrite));
	}

	#[test]
	fn test_bitand() {
		let subset = Seal::Seal | Seal::Write;
		assert!(Seals::all() & subset == subset);
		assert!((Seals::all() & subset).len() == 2);
	}

	#[test]
	fn test_bitxor() {
		assert!(Seals::all() ^ (Seal::Seal | Seal::Write) == (Seal::Shrink | Seal::Grow | Seal::FutureWrite));
	}

	#[test]
	fn test_debug() {
		assert!(format!("{:?}", Seals::empty()) == "Seals { }");
		assert!(format!("{:?}", Seals::from(Seal::Seal)) == "Seals { Seal }");
		assert!(format!("{:?}", Seal::Seal | Seal::Shrink) == "Seals { Seal | Shrink }");
		assert!(format!("{:?}", Seals::all()) == "Seals { Seal | Shrink | Grow | Write | FutureWrite }");
	}
}
