use vmm_ir::{Instruction, Offset, ScaleAnd, SuperInstruction};
use vmm_num::ops::WrappingMul;

use crate::{Change, PeepholePass};

#[derive(Debug, Default)]
pub struct OptimizeSetScaleValPass;

impl PeepholePass for OptimizeSetScaleValPass {
	const SIZE: usize = 2;

	#[inline]
	fn run_pass(&mut self, window: &[Instruction]) -> Option<Change> {
		match window {
			[
				Instruction::SetVal {
					value: Some(value),
					offset: Offset(0),
				},
				Instruction::ScaleVal { factor },
			] => Some(Change::replace(Instruction::set_val(
				WrappingMul::wrapping_mul(value.get(), *factor),
			))),
			[
				Instruction::SetVal {
					value: Some(value),
					offset: Offset(0),
				},
				Instruction::Super(SuperInstruction::ScaleAnd {
					action: ScaleAnd::Take,
					factor,
					offset,
				}),
			] => Some(Change::swap([
				Instruction::clear_val(),
				Instruction::move_ptr(*offset),
				Instruction::inc_val(WrappingMul::wrapping_mul(value.get(), factor) as i8),
			])),
			[
				Instruction::SetVal {
					value: Some(value),
					offset: Offset(0),
				},
				Instruction::Super(SuperInstruction::ScaleAnd {
					action: ScaleAnd::Move,
					offset,
					factor,
				}),
			] => Some(Change::swap([
				Instruction::clear_val(),
				Instruction::inc_val_at(
					WrappingMul::wrapping_mul(value.get(), factor) as i8,
					*offset,
				),
			])),
			_ => None,
		}
	}

	#[inline]
	fn should_run(&self, window: &[Instruction]) -> bool {
		matches!(
			window,
			[
				Instruction::SetVal {
					value: Some(..),
					offset: Offset(0)
				},
				Instruction::ScaleVal { .. }
					| Instruction::Super(SuperInstruction::ScaleAnd {
						action: ScaleAnd::Take | ScaleAnd::Move,
						..
					})
			]
		)
	}
}
