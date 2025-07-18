#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

extern crate alloc;

mod block_instr;
mod hint;
mod offset;
mod super_instr;
mod utils;

use alloc::string::ToString;
use core::{
	fmt::{Debug, Display, Formatter, Result as FmtResult},
	num::NonZeroU8,
};

use serde::{Deserialize, Serialize};
use tap::prelude::*;
use vmm_utils::GetOrZero as _;

pub use self::{block_instr::*, hint::*, offset::*, super_instr::*, utils::*};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Instruction {
	/// The "boundary" (start/end) of a program
	/// Is a no-op, but allows for other optimizations to be applied
	Boundary,
	/// Increment the value at the current cell (offset = None) or at an offset
	IncVal {
		value: i8,
		offset: Offset,
	},
	SubCell {
		offset: Offset,
	},
	/// Set the value at the current cell (offset = None) or at an offset
	SetVal {
		value: Option<NonZeroU8>,
		offset: Offset,
	},
	/// Multiply self by factor
	ScaleVal {
		factor: u8,
	},
	MoveVal(Offset),
	FetchVal(Offset),
	TakeVal(Offset),
	ReplaceVal(Offset),
	/// Move the pointer along the tape
	MovePtr(Offset),
	/// Find the next zero, jumping by the value
	FindZero(Offset),
	/// Read a value from the input
	Read,
	/// Write the value to an output
	Write {
		offset: Offset,
	},
	/// A block of instructions
	Block(BlockInstruction),
	/// A "Super" instruction, which is an instruction that does more than one action
	Super(SuperInstruction),
}

impl Instruction {
	#[must_use]
	pub fn inc_val(v: i8) -> Self {
		Self::inc_val_at(v, 0)
	}

	#[must_use]
	pub const fn changes_current_cell(&self) -> bool {
		matches!(
			self,
			Self::SetVal {
				offset: Offset(0),
				..
			} | Self::IncVal {
				offset: Offset(0),
				..
			}
		)
	}

	#[must_use]
	pub fn sub_cell(offset: impl Into<Offset>) -> Self {
		Self::SubCell {
			offset: offset.convert(),
		}
	}

	#[must_use]
	pub fn scale_and_take_val(factor: u8, offset: impl Into<Offset>) -> Self {
		SuperInstruction::scale_and_take_val(factor, offset).convert()
	}

	#[must_use]
	pub fn inc_val_at(v: i8, offset: impl Into<Offset>) -> Self {
		Self::IncVal {
			value: v.convert(),
			offset: offset.convert(),
		}
	}

	#[must_use]
	pub fn find_and_set_zero(v: NonZeroU8, offset: impl Into<Offset>) -> Self {
		Self::Super(SuperInstruction::find_and_set_zero(v, offset))
	}

	#[must_use]
	pub fn set_val(v: u8) -> Self {
		Self::set_val_at(v, 0)
	}

	#[must_use]
	pub fn set_val_at(v: u8, offset: impl Into<Offset>) -> Self {
		Self::SetVal {
			value: NonZeroU8::new(v),
			offset: offset.convert(),
		}
	}

	#[must_use]
	pub fn clear_val() -> Self {
		Self::clear_val_at(0)
	}

	#[must_use]
	pub fn clear_val_at(offset: impl Into<Offset>) -> Self {
		Self::SetVal {
			value: None,
			offset: offset.convert(),
		}
	}

	#[must_use]
	pub fn move_val(offset: impl Into<Offset>) -> Self {
		Self::MoveVal(offset.convert())
	}

	#[must_use]
	pub fn fetch_val(offset: impl Into<Offset>) -> Self {
		Self::FetchVal(offset.convert())
	}

	#[must_use]
	pub fn take_val(offset: impl Into<Offset>) -> Self {
		Self::TakeVal(offset.convert())
	}

	#[must_use]
	pub fn replace_val(offset: impl Into<Offset>) -> Self {
		Self::ReplaceVal(offset.convert())
	}

	#[must_use]
	pub fn scale_and_move_val(factor: u8, offset: impl Into<Offset>) -> Self {
		SuperInstruction::scale_and_move_val(factor, offset).convert()
	}

	#[must_use]
	pub fn fetch_and_scale_val(factor: u8, offset: impl Into<Offset>) -> Self {
		SuperInstruction::fetch_and_scale_val(factor, offset).convert()
	}

	#[must_use]
	pub fn scale_and_set_val(factor: u8, offset: impl Into<Offset>, value: NonZeroU8) -> Self {
		SuperInstruction::scale_and_set_val(factor, offset, value).convert()
	}

	#[must_use]
	pub fn move_ptr(offset: impl Into<Offset>) -> Self {
		Self::MovePtr(offset.convert())
	}

	#[must_use]
	pub const fn move_ptr_by(offset: isize) -> Self {
		Self::MovePtr(Offset::new(offset))
	}

	#[must_use]
	pub fn find_zero(jump_by: impl Into<Offset>) -> Self {
		Self::FindZero(jump_by.convert())
	}

	#[must_use]
	pub const fn read() -> Self {
		Self::Read
	}

	#[inline]
	#[must_use]
	pub fn write_once() -> Self {
		Self::write_once_at(0)
	}

	#[inline]
	#[must_use]
	pub fn write_once_at(offset: impl Into<Offset>) -> Self {
		// Self::write_value(Value::<Bytes>::CellAt(offset.convert()))
		Self::Write {
			offset: offset.into(),
		}
	}

	#[must_use]
	pub const fn scale_val(factor: u8) -> Self {
		Self::ScaleVal { factor }
	}

	#[must_use]
	pub fn dynamic_loop(instructions: impl IntoIterator<Item = Self>) -> Self {
		BlockInstruction::dynamic(instructions).convert()
	}

	#[must_use]
	pub fn if_nz(instructions: impl IntoIterator<Item = Self>) -> Self {
		BlockInstruction::if_nz(instructions).convert()
	}

	#[must_use]
	pub fn set_until_zero(value: u8, offset: impl Into<Offset>) -> Self {
		Self::Super(SuperInstruction::set_until_zero(value, offset))
	}

	#[must_use]
	pub fn find_cell_by_zero(jump_by: impl Into<Offset>, offset: impl Into<Offset>) -> Self {
		Self::Super(SuperInstruction::find_cell_by_zero(jump_by, offset))
	}

	#[must_use]
	pub fn shift_vals(jump_by: impl Into<Offset>, offset: impl Into<Offset>) -> Self {
		Self::Super(SuperInstruction::shift_vals(jump_by, offset))
	}

	#[must_use]
	pub const fn is_overwriting_current_cell(&self) -> bool {
		matches!(
			self,
			Self::SetVal {
				offset: Offset(0),
				..
			} | Self::Read
				| Self::Super(SuperInstruction::ScaleAnd {
					action: ScaleAnd::Move,
					..
				}) | Self::Block(BlockInstruction::IfNz(..))
				| Self::MoveVal(..)
		)
	}

	#[must_use]
	pub const fn is_change_val(&self) -> bool {
		matches!(self, Self::IncVal { .. })
	}

	#[must_use]
	pub const fn is_set_val(&self) -> bool {
		matches!(self, Self::SetVal { .. })
	}

	#[must_use]
	pub const fn is_clear_val(&self) -> bool {
		matches!(self, Self::SetVal { value: None, .. })
	}

	#[must_use]
	pub const fn is_move_val(&self) -> bool {
		matches!(
			self,
			Self::Super(SuperInstruction::ScaleAnd {
				action: ScaleAnd::Move,
				..
			}) | Self::MoveVal(..)
		)
	}

	#[must_use]
	pub const fn is_dynamic_loop(&self) -> bool {
		matches!(self, Self::Block(BlockInstruction::DynamicLoop(_)))
	}

	#[must_use]
	pub fn is_empty_dynamic_loop(&self) -> bool {
		matches!(self, Self::Block(BlockInstruction::DynamicLoop(l)) if l.is_empty())
	}

	pub fn rough_estimate(&self) -> usize {
		match self {
			Self::Block(BlockInstruction::DynamicLoop(l)) => {
				l.iter().map(Self::rough_estimate).sum::<usize>() + 2
			}
			Self::Block(BlockInstruction::IfNz(l)) => {
				l.iter().map(Self::rough_estimate).sum::<usize>() + 1
			}
			_ => 1,
		}
	}

	#[must_use]
	pub fn raw_rough_estimate(&self) -> usize {
		self.to_string().len()
	}

	#[must_use]
	pub const fn offset(&self) -> Option<Offset> {
		match self {
			Self::SetVal { offset, .. } | Self::IncVal { offset, .. } => Some(*offset),
			_ => None,
		}
	}

	#[inline]
	#[must_use]
	pub fn nested_loops(&self) -> usize {
		let mut count = 0;

		if let Self::Block(instrs) = self {
			count += 1;

			instrs.iter().for_each(|instr| {
				count += instr.nested_loops();
			});
		}

		count
	}
}

impl Display for Instruction {
	#[allow(unreachable_patterns)] // For when we add more instructions.
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::IncVal { value, offset } => {
				write!(f, "inc {value}")?;
				if !offset.is_zero() {
					write!(f, " {offset:#}")?;
				}
			}
			Self::SetVal { value, offset } => {
				write!(f, "set {}", value.get_or_zero())?;

				if !offset.is_zero() {
					write!(f, " {offset:#}")?;
				}
			}
			Self::MovePtr(offset) => {
				write!(f, "mov(ptr) {offset:#}")?;
			}
			Self::ScaleVal { factor } => {
				write!(f, "scale {factor}")?;
			}
			Self::FindZero(offset) => {
				write!(f, "findz {offset:#}")?;
			}
			Self::MoveVal(offset) => {
				write!(f, "mov(val) {offset:#}")?;
			}
			Self::FetchVal(offset) => {
				write!(f, "fetch(val) {offset:#}")?;
			}
			Self::TakeVal(offset) => {
				write!(f, "take(val) {offset:#}")?;
			}
			Self::Read => f.write_str("getc")?,
			Self::Boundary => f.write_str("boundary")?,
			Self::Super(s) => Display::fmt(&s, f)?,
			Self::Block(l) => Display::fmt(&l, f)?,
			Self::Write { offset } => {
				f.write_str("putc")?;

				if !offset.is_zero() {
					write!(f, " {offset:#}")?;
				}
			}
			i => Debug::fmt(&i, f)?,
		}

		Ok(())
	}
}

impl From<BlockInstruction> for Instruction {
	fn from(value: BlockInstruction) -> Self {
		Self::Block(value)
	}
}

impl From<SuperInstruction> for Instruction {
	fn from(value: SuperInstruction) -> Self {
		Self::Super(value)
	}
}

impl HasIo for Instruction {
	fn has_read(&self) -> bool {
		match self {
			Self::Block(b) => b.has_read(),
			Self::Super(s) => s.has_read(),
			Self::Read => true,
			_ => false,
		}
	}

	fn has_write(&self) -> bool {
		match self {
			Self::Block(b) => b.has_write(),
			Self::Super(s) => s.has_write(),
			Self::Write { .. } => true,
			_ => false,
		}
	}
}

impl IsOffsetable for Instruction {
	fn is_offsetable(&self) -> bool {
		// matches!(self, Self::IncVal { .. } | Self::SetVal { .. })
		match self {
			Self::Block(b) => b.is_offsetable(),
			Self::Super(s) => s.is_offsetable(),
			Self::IncVal { .. } | Self::SetVal { .. } => true,
			_ => false,
		}
	}

	fn offset(&self) -> Option<Offset> {
		match self {
			Self::Block(b) => b.offset(),
			Self::Super(s) => s.offset(),
			Self::IncVal { offset, .. } | Self::SetVal { offset, .. } => Some(*offset),
			_ => None,
		}
	}

	fn set_offset(&mut self, offset: Offset) {
		match self {
			Self::IncVal {
				offset: self_offset,
				..
			}
			| Self::SetVal {
				offset: self_offset,
				..
			} => *self_offset = offset,
			Self::Block(b) => b.set_offset(offset),
			Self::Super(s) => s.set_offset(offset),
			_ => {}
		}
	}
}

impl IsZeroingCell for Instruction {
	#[inline]
	fn is_zeroing_cell(&self) -> bool {
		match self {
			Self::Block(l) => l.is_zeroing_cell(),
			Self::Super(s) => s.is_zeroing_cell(),
			Self::SetVal {
				value: None,
				offset: Offset(0),
			}
			| Self::MoveVal(..)
			| Self::FindZero(..)
			| Self::SubCell { .. } => true,
			_ => false,
		}
	}
}

impl MinimumOutputs for Instruction {
	fn min_outputs(&self) -> usize {
		match self {
			Self::Block(b) => b.min_outputs(),
			Self::Super(s) => s.min_outputs(),
			Self::Write { .. } => 1,
			_ => 0,
		}
	}
}

impl PtrMovement for Instruction {
	#[inline]
	fn ptr_movement(&self) -> Option<Offset> {
		match self {
			Self::Super(s) => s.ptr_movement(),
			Self::Block(l) => l.ptr_movement(),
			Self::ScaleVal { .. }
			| Self::SetVal { .. }
			| Self::IncVal { .. }
			| Self::Boundary
			| Self::Read
			| Self::SubCell { .. }
			| Self::FetchVal(..)
			| Self::MoveVal(..)
			| Self::ReplaceVal(..)
			| Self::Write { .. } => Some(Offset(0)),
			Self::MovePtr(offset) | Self::TakeVal(offset) => Some(*offset),
			_ => None,
		}
	}
}
