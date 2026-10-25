use std::{
	cell::{Cell, Ref, RefCell},
	sync::{
		Arc,
		atomic::{AtomicI64, Ordering},
	},
	thread,
	time::Duration,
};

use evloop_core::{
	data_transfer::{DataTransferId, DndAction},
	error::TransferError,
};
use windows_sys::Win32::{
	Foundation::{CLIPBRD_E_CANT_OPEN, S_OK},
	System::{
		DataExchange::GetClipboardSequenceNumber,
		Ole::{DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE, OleFlushClipboard},
	},
};

use crate::data_transfer::{drop_effect::DropEffect, receive::snapshot::DataSnapshot};

pub(crate) fn next_data_transfer_id() -> DataTransferId {
	static COUNTER: AtomicI64 = AtomicI64::new(0);
	DataTransferId::from_raw(COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[derive(Debug)]
pub(crate) struct DragState {
	id: DataTransferId,
	window_id: WindowId,
	data: Arc<DataSnapshot>,
	actions: Vec<DndAction>,
}

#[derive(Debug)]
struct ClipboardState {
	sequence: u32,
	id: DataTransferId,
	data: Option<Arc<DataSnapshot>>,
}

impl ClipboardState {
	fn is_current(&self, id: DataTransferId) -> bool {
		self.id == id && self.sequence == unsafe { GetClipboardSequenceNumber() }
	}
}

#[derive(Debug, Default)]
pub(crate) struct DataTransferState {
	drag: RefCell<Option<DragState>>,
	clipboard: RefCell<Option<ClipboardState>>,
	owns_clipboard: Cell<bool>,
}

impl DataTransferState {
	pub(crate) fn register_drag(
		&self,
		id: DataTransferId,
		window_id: WindowId,
		data: Arc<DataSnapshot>,
	) {
		*self.drag.borrow_mut() =
			Some(DragState { id, window_id, data, actions: Default::default() });
	}

	pub(crate) fn remove_drag(&self, id: DataTransferId) {
		let mut state = self.drag.borrow_mut();
		if state.as_ref().is_some_and(|s| s.id == id) {
			*state = None;
		}
	}

	pub(crate) fn drag_state(&self, id: DataTransferId) -> Option<Ref<'_, DragState>> {
		Ref::filter_map(self.drag.borrow(), |state| state.as_ref().filter(|s| s.id == id)).ok()
	}

	pub(crate) fn current_drag_actions(&self, id: DataTransferId) -> Ref<'_, [DndAction]> {
		Ref::map(self.drag.borrow(), |state| {
			state.as_ref().filter(|s| s.id == id).map(|s| &s.actions[..]).unwrap_or_default()
		})
	}

	pub(crate) fn set_valid_dnd_actions(&self, id: DataTransferId, actions: &[DndAction]) -> bool {
		match &mut *self.drag.borrow_mut() {
			Some(state) if state.id == id => {
				state.actions.clear();
				state.actions.extend_from_slice(actions);
				true
			}
			_ => false,
		}
	}

	pub(crate) fn proposed_dnd_action(
		&self,
		id: DataTransferId,
		effects: DropEffect,
	) -> Option<DndAction> {
		self.current_drag_actions(id).iter().copied().find(|action| {
			let effect = match action {
				DndAction::Move => DROPEFFECT_MOVE,
				DndAction::Copy => DROPEFFECT_COPY,
				DndAction::Link => DROPEFFECT_LINK,
				_ => return false,
			};

			(effects & effect) != 0
		})
	}

	pub(crate) fn clipboard_transfer_id(&self) -> DataTransferId {
		let sequence = unsafe { GetClipboardSequenceNumber() };
		let mut clipboard = self.clipboard.borrow_mut();
		clipboard.take_if(|state| state.sequence != sequence);
		clipboard
			.get_or_insert_with(|| ClipboardState {
				sequence,
				id: next_data_transfer_id(),
				data: None,
			})
			.id
	}

	pub(crate) fn clipboard_data(
		&self,
		id: DataTransferId,
	) -> Result<Arc<DataSnapshot>, TransferError> {
		match &*self.clipboard.borrow() {
			Some(state) if state.is_current(id) => {
				if let Some(data) = &state.data {
					return Ok(data.clone());
				}
			}
			_ => return Err(TransferError::UnknownTransfer(id)),
		}

		let data = Arc::new(DataSnapshot::from_clipboard()?);

		match &mut *self.clipboard.borrow_mut() {
			Some(state) if state.is_current(id) => {
				state.data = Some(data.clone());
				Ok(data)
			}
			_ => Err(TransferError::UnknownTransfer(id)),
		}
	}

	pub(crate) fn set_owns_clipboard(&self) {
		self.owns_clipboard.set(true)
	}

	pub(crate) fn flush_clipboard_blocking(&self) {
		if !self.owns_clipboard.replace(false) {
			return;
		}

		let mut hr = unsafe { OleFlushClipboard() };
		for _ in 0..4 {
			if hr != CLIPBRD_E_CANT_OPEN {
				break;
			}
			thread::sleep(Duration::from_millis(10));
			hr = unsafe { OleFlushClipboard() };
		}

		if hr != S_OK {
			tracing::warn!("Failed to flush clipboard: 0x{hr:08x}");
		}
	}

	pub(crate) fn reset(&self) {
		self.drag.take();
		self.clipboard.take();
	}
}
