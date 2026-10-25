use std::{num::NonZeroU32, ops::BitOr};

use evloop_core::data_transfer::DndAction;
use windows_sys::Win32::System::Ole::{
	DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE, DROPEFFECT_NONE,
};

pub(crate) type DropEffect = u32;

pub(crate) fn dnd_action_to_dropeffect_mask(action: DndAction) -> DropEffect {
	match action {
		DndAction::Move => DROPEFFECT_MOVE,
		DndAction::Copy => DROPEFFECT_COPY,
		DndAction::Link => DROPEFFECT_LINK,
		_ => DROPEFFECT_NONE,
	}
}

pub(crate) fn dnd_actions_to_dropeffect_mask(actions: &[DndAction]) -> DropEffect {
	actions
		.iter()
		.copied()
		.map(dnd_action_to_dropeffect_mask)
		.fold(DropEffect::default(), BitOr::bitor)
}

pub(crate) fn drop_effect_to_dnd_action(effect: DropEffect) -> Option<DndAction> {
	match effect {
		DROPEFFECT_MOVE => Some(DndAction::Move),
		DROPEFFECT_COPY => Some(DndAction::Copy),
		DROPEFFECT_LINK => Some(DndAction::Link),
		_ => None,
	}
}

pub(crate) fn pick_effect(actions: &[DndAction], key_state: u32, source_allowed: u32) -> u32 {
	const MK_SHIFT: u32 = 0x0004;
	const MK_CONTROL: u32 = 0x0008;

	// from(winit-win32): TODO: On macOS, option will disable copy (so if copy has higher precedence than move you can
	// select move via option). Is alt expected to act similarly on Windows?
	let mut allowed = dnd_actions_to_dropeffect_mask(actions) & source_allowed;
	if allowed == 0 {
		return DROPEFFECT_NONE;
	}

	if let Some(new_allowed) =
		NonZeroU32::new(allowed & !DROPEFFECT_MOVE).filter(|_| key_state & MK_CONTROL != 0)
	{
		allowed = new_allowed.get();
	}

	if let Some(new_allowed) =
		NonZeroU32::new(allowed & !DROPEFFECT_COPY).filter(|_| key_state & MK_SHIFT != 0)
	{
		allowed = new_allowed.get();
	}

	for action in actions {
		let effect = dnd_action_to_dropeffect_mask(*action);
		if (allowed & effect) != 0 {
			return effect;
		}
	}

	DROPEFFECT_NONE
}
