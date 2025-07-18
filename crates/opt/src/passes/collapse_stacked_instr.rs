use vmm_ir::{Instruction, Offset};
use vmm_num::ops::{WrappingAdd, WrappingMul};
use vmm_utils::GetOrZero as _;

use crate::{Change, PeepholePass};

#[derive(Debug, Default)]
pub struct CollapseStackedInstrPass;

impl PeepholePass for CollapseStackedInstrPass {
	const SIZE: usize = 2;

	#[inline]
	#[allow(clippy::many_single_char_names)]
	fn run_pass(&mut self, window: &[Instruction]) -> Option<Change> {
		match window {
			[
				Instruction::IncVal {
					value: i1,
					offset: Offset(0),
				},
				Instruction::IncVal {
					value: i2,
					offset: Offset(0),
				},
			] => Some(Change::replace(Instruction::inc_val(
				WrappingAdd::wrapping_add(i1, i2),
			))),
			[Instruction::MovePtr(i1), Instruction::MovePtr(i2)] if *i1 == -i2 => {
				Some(Change::remove())
			}
			[Instruction::MovePtr(i1), Instruction::MovePtr(i2)] => Some(Change::replace(
				Instruction::move_ptr(WrappingAdd::wrapping_add(i1, i2)),
			)),
			[
				Instruction::SetVal {
					offset: Offset(0), ..
				},
				Instruction::SetVal {
					value: Some(x),
					offset: Offset(0),
				},
			] => Some(Change::replace(Instruction::set_val(x.get()))),
			[
				Instruction::IncVal {
					value: i1,
					offset: x,
				},
				Instruction::IncVal {
					value: i2,
					offset: y,
				},
			] if x == y => Some(Change::replace(Instruction::inc_val_at(
				WrappingAdd::wrapping_add(i1, i2),
				*x,
			))),
			[
				Instruction::SetVal {
					value: None,
					offset: Offset(0),
				},
				Instruction::SetVal {
					value: None,
					offset: Offset(0),
				},
			] => Some(Change::remove_offset(1)),
			[
				Instruction::SetVal {
					value: None,
					offset: x,
				},
				Instruction::SetVal {
					value: None,
					offset: y,
				},
			] if *x == *y => Some(Change::remove_offset(1)),
			[
				Instruction::SetVal { offset: x, .. },
				Instruction::SetVal { offset: y, value },
			] if *x == *y => Some(Change::replace(Instruction::set_val_at(
				value.get_or_zero(),
				*x,
			))),
			[
				Instruction::SetVal {
					offset: Offset(0), ..
				},
				Instruction::SetVal {
					value,
					offset: Offset(0),
				},
			] => Some(Change::replace(Instruction::set_val(value.get_or_zero()))),
			[
				Instruction::ScaleVal { factor: x },
				Instruction::ScaleVal { factor: y },
			] => Some(Change::Replace(Instruction::scale_val(
				WrappingMul::wrapping_mul(x, y),
			))),
			[Instruction::TakeVal(l), Instruction::TakeVal(r)] => {
				Some(Change::replace(Instruction::take_val(l + r)))
			}
			_ => None,
		}
	}

	#[inline]
	fn should_run(&self, window: &[Instruction]) -> bool {
		matches!(
			window,
			[
				Instruction::IncVal {
					offset: Offset(0),
					..
				},
				Instruction::IncVal {
					offset: Offset(0),
					..
				}
			] | [Instruction::MovePtr(..), Instruction::MovePtr(..)]
				| [
					Instruction::SetVal {
						offset: Offset(0),
						..
					},
					Instruction::SetVal {
						offset: Offset(0),
						..
					}
				] | [Instruction::ScaleVal { .. }, Instruction::ScaleVal { .. }]
				| [Instruction::TakeVal(..), Instruction::TakeVal(..)]
		) || matches!(
			window,
			[
				Instruction::IncVal { offset: x, .. },
				Instruction::IncVal { offset: y, .. }
			] | [
				Instruction::SetVal { offset: x, .. },
				Instruction::SetVal { offset: y, .. }
			]
			if *x == *y
		)
	}
}
